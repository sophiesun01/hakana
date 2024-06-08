namespace Cache;

use type AsyncKeyBatcher, CacheResult, cache_source_t;
use type Logger\Log;
use namespace HH\Lib\{C, Dict, Keyset};

use function \{job_queue_enqueue};

/**
 * Contains the Cache class that is the wrapper and entry point to all things cache. This class is meant to be a base class to be extended for your
 * own caches you wish to implement.
 *
 * For info on how to implement a cache please see https://slack-github.com/slack/docs/blob/master/appeng/cache.md
 */

/**
 * Metadata describing a cache.
 */
interface ICacheMetadata {
	abstract const CachePrefix PREFIX;
	abstract const cache_config_t<arraykey> CONFIG;
}

/**
 * Wrapper for all cached data operations.
 *
 * Based on the configuration, values can be cached per request, per host, or remotely in memcached (or any combination of these).
 * Subclasses define configuration based on key prefix, such as which caches to use and TTLs
 *
 * To implement a cache:
 * - Create a final class that extends Cache\Cache<TSuffix, TData>
 * - For TSuffix, enter the type of the key (for example, int for user ids). For TData put the type of the values.
 * - Define PREFIX and CONFIG. Detailed documentation in `src/cache/cache_config.hack` explains how to choose each configuration option.
 *
 * To use a cache:
 * - Get an instance with `MyCacheClass::getInstance()`
 * - Use `getAsync` with a fill function to get values and set them when missing.
 * - Use `unsetAsync` to remove values from cache.
 *
 * For more see https://slack-github.com/slack/docs/blob/master/appeng/cache.md
 */
<<__ConsistentConstruct>>
abstract class Cache<TSuffix as arraykey, TData> implements ICacheMetadata {

	// defined by subclasses
	abstract const CachePrefix PREFIX;
	abstract const cache_config_t<TSuffix> CONFIG;

	// populated in constructor depending on config
	protected ?PerRequestCache<TSuffix, TData> $localCache;
	protected ?RemoteCache<TSuffix> $remoteCache;
	protected ?PerHostCache $hostCache;
	protected static keyset<classname<Cache<arraykey, mixed>>> $instances = keyset[];

	// used to avoid concurrently filling the same key
	protected dict<TSuffix, Awaitable<?TData>> $inProgressFills = dict[];

	// used by getBatched
	const int MAX_FILL_BATCH_SIZE = 1000;
	protected ?AsyncKeyBatcher<TSuffix, TData> $fillBatcher = null;

	protected function __construct() {
		if ($this::CONFIG['use_local'] ?? true) {
			$this->localCache = new PerRequestCache<TSuffix, TData>($this::CONFIG['local_ttl'] ?? null);
		}

		if ($this::CONFIG['remote_ttl'] ?? false) {
			$this->remoteCache = new RemoteCache($this::CONFIG);
		}

		if ($this::CONFIG['host_cache_ttl'] ?? false) {
			$this->hostCache = \resolve<PerHostCache>();
		}
	}

	/**
	 * Get the singleton instance of this Cache class.
	 *
	 * The `final` constructor for Cache, along with the memoized attribute on this method ensures
	 * there is only ever one instance per subclass. MemoizeLSB (late static binding) ensures each
	 * subclass is memoized separately.
	 */
	<<__MemoizeLSB>>
	public static function getInstance(): this {
		$instance = new static();
		self::$instances[] = \get_class($instance);
		return $instance;
	}

	/**
	 * Clear all per-request/in-memory caches.
	 *
	 * This is for testing purposes. In production, per-request caches are automatically cleared
	 * between requests.
	 *
	 * Note that this does not affect the test version of per-host caches or remote caches.
	 * To clear those, use resetAllCachesForTests.
	 */
	<<\Hakana\TestOnly>>
	public static function resetPerRequestCachesForTests(): void {
		// TODO: Consider moving to a class that is explicitly test-only
		static::resetAllInstances();
	}

	/**
	 * Clear all per-request, per-host, and remote caches.
	 *
	 * This is for testing purposes. In production, per-request caches are automatically cleared
	 * between requests, but per-host and remote caches persist.
	 */
	<<\Hakana\TestOnly>>
	public static function resetAllCachesForTests(): void {
		// TODO: Consider moving to a class that is explicitly test-only
		static::resetAllInstances();
		MCRouterMock::getInstance()->reset();
		(\resolve<PerHostCache>() as ApcMock)->reset();
		Metrics::remoteCacheCount()->reset();
	}

	/**
	 * Clear all per-request/in-memory caches.
	 *
	 * This is for testing purposes. In production, per-request caches are automatically cleared
	 * between requests. API integration tests use this method to clear per-request caches
	 * before requests.
	 *
	 * Note that this does not affect the test version of per-host caches or remote caches.
	 * To clear those, use resetAllCachesForTests.
	 *
	 * This is the old name for resetAllPerRequestCachesForTests
	 * Prefer resetAllPerRequestCachesForTests or resetAllForTests
	 */
	<<\Hakana\TestOnly>>
	public static function resetAllInstances(): void {
		foreach (self::$instances as $class) {
			$class::getInstance()->clearLocalCache();

			// Note: this resets the cache by discarding the instance returned by getInstance.
			//
			// While this is a reliable way to completely reset state, it breaks the singleton
			// contract of class. Tests have references to the old instance which are no longer
			// valid after this call.
			//
			// TODO: Stop calling clear_lsb_memoization, fix failing tests, and
			// rely on clearLocalCache
			\HH\clear_lsb_memoization($class);
		}
	}

	/**
	* Should we gather metrics for usage of the per host cache? Sample at 1%.
	*/
	<<__Memoize>>
	public static function shouldComputeHostCacheMetrics(): bool {
		return \ExecutionContext::is_test() || \util_percent_chance(1);
	}

	/**
	 * Should we gather metrics for usage of local cache?
	 * We only do this on 1% of requests
	 */
	<<__Memoize>>
	private static function shouldComputeLocalCacheMetrics(): bool {
		return \ExecutionContext::is_test() || \util_percent_chance(1);
	}

	// Ignoring these cache paths speeds up analysis, and I don't think tainted data can really easily flow here
	<<\Hakana\SecurityAnalysis\IgnorePath()>>
	/**
	 * Get from the tier of possible caches, calling a fill function if all miss
	 * Writes back to caches after fill function is called
	 *
	 * The fill function is optional, but recommended for most uses, since it will handle
	 * setting the cache after filling. An example of when not to fill would be if you want to
	 * try multiple cache keys before running a fill function.
	 *
	 * If $expected_type is passed in, it will be verified on fetches from remote or host-level caches.
	 * This does incur a performance cost.
	 */
	final public async function getAsync(
		TSuffix $suffix,
		?(function(TSuffix): Awaitable<?TData>) $fill_fn = null,
		?typename<TData> $expected_type = null,
	): Awaitable<CacheResult<TData>> {

		$local_cache = $this->localCache;
		$remote_cache = $this->remoteCache;
		$host_cache = $this->hostCache;
		$host_cache_key = null;
		// if we fill cache results, should we set those back in the cache?
		// true unless we encountered an error communicating with the remote cache in this request,
		// since our ability to talk to the remote cache may be compromised
		$should_set_remote = true;
		$result = null;

		$expected_type = $this->selectExpectedDataType($expected_type);

		if ($local_cache is nonnull) {
			$result = $local_cache->get($suffix);
			if (self::shouldComputeLocalCacheMetrics()) {
				Metrics::requestCacheCount()->inc(
					1,
					shape(
						'op' => 'get',
						'prefix' => $this::PREFIX,
						'result' => $result->resultLabel(),
						'library' => CacheLibraries::SRC_CACHE,
					),
				);
			}
			if ($result->hit()) {
				return $result;
			}
		}

		if ($host_cache is nonnull) {
			$host_cache_key = $this->buildHostCacheKey($this::CONFIG, $this::PREFIX, $suffix);
			$result = $expected_type is null
				? $host_cache->get($host_cache_key)
				: $host_cache->getTyped($host_cache_key, $expected_type);
			if (self::shouldComputeHostCacheMetrics()) {
				Metrics::hostCacheCount()->inc(
					1,
					shape(
						'op' => 'get',
						'prefix' => $this::PREFIX,
						'result' => $result->resultLabel(),
						'library' => CacheLibraries::SRC_CACHE,
					),
				);
			}

			if ($this->shouldWriteBack(/* HH_FIXME[4110] unsafe nested mixed */ $result)) {
				if ($local_cache is nonnull) {
					// Use maybeData here because a negatively cached hit won't have nonnull data
					$local_cache->set(
						$suffix,
						/* HH_FIXME[4110] unsafe nested mixed */ $result->data()
					);
					if (self::shouldComputeLocalCacheMetrics()) {
						Metrics::requestCacheCount()->inc(
							1,
							shape(
								'op' => 'set',
								'prefix' => $this::PREFIX,
								'result' => 'ok',
								'library' => CacheLibraries::SRC_CACHE,
							),
						);
					}
				}

				/* HH_FIXME[4110] unsafe nested mixed */
				return $result;
			}
		}

		// Don't fetch the same key twice concurrently
		if ($fill_fn) {
			$fill_fn = async (TSuffix $suffix) ==> {
				$this->inProgressFills[$suffix] ??= $fill_fn($suffix);
				$ret = await $this->inProgressFills[$suffix];
				// This is fine even if another call has already unset it
				unset($this->inProgressFills[$suffix]);
				return $ret;
			};
		}

		//
		// check remote
		//

		if ($remote_cache is nonnull) {
			if (($this::CONFIG['lock_strategy'] !== LockStrategy::NONE) && $fill_fn is nonnull) {
				$result = await $remote_cache->getWithLockAsync($this::PREFIX, $suffix, $fill_fn, $expected_type);
				if (!$result->hit() && $this::CONFIG['lock_strategy'] === LockStrategy::LOCK_OR_MISS) {
					// when using locks, if we failed to get the data we don't want to invoke the fill function below
					// to avoid overwhelming the backing store
					return $result;
				} else if (!$result->hit()) {
					// we didn't acquire a lock. don't set the remote cache even though we're going to fill
					$should_set_remote = false;
				}
			} else {
				$result = await $remote_cache->getAsync($this::PREFIX, $suffix, $expected_type);
			}

			if ($this->shouldWriteBack($result)) {
				if ($host_cache is nonnull && $host_cache_key is nonnull) {
					$host_cache->set($host_cache_key, $result->data(), $this->hostCacheTTL());
					if (self::shouldComputeHostCacheMetrics()) {
						Metrics::hostCacheCount()->inc(
							1,
							shape(
								'op' => 'set',
								'prefix' => $this::PREFIX,
								'result' => 'ok',
								'library' => CacheLibraries::SRC_CACHE,
							),
						);
					}
				}

				if ($local_cache is nonnull) {
					$local_cache->set($suffix, $result->data());
					if (self::shouldComputeLocalCacheMetrics()) {
						Metrics::requestCacheCount()->inc(
							1,
							shape(
								'op' => 'set',
								'prefix' => $this::PREFIX,
								'result' => 'ok',
								'library' => CacheLibraries::SRC_CACHE,
							),
						);
					}
				}
				return $result;
			} else if ($result->isError()) {
				// if we got an error on any cache gets, we don't set remote cache for the rest of the request
				$should_set_remote = false;
			}
		}

		// we missed all caches. if no fill function was provided, we're done
		if ($fill_fn is null) {
			// return the cache $result from the last cache operation so callers can differentiate between error and miss
			invariant($result is nonnull, 'at least one cache must exist in Cache configuration');
			/* HH_FIXME[4110] unsafe nested mixed */
			return $result;
		}

		// We can use config deploy to quickly stop cache from filling a given prefix
		// This is used as a guard rail against sudden load and is meant to be used during incidents
		$short_circuit_config = CacheShortCircuitConfigLoader::loadConfig();
		if ($short_circuit_config->shouldSkipFill(static::PREFIX)) {
			// return the cache $result from the last cache operation so callers can differentiate between error and miss
			invariant($result is nonnull, 'at least one cache must exist in Cache configuration');
			/* HH_FIXME[4110] unsafe nested mixed */
			return $result;
		}

		// let's get the data from source
		if ($remote_cache is nonnull) {
			CacheFillTracker::set(static::PREFIX);
		}
		$data = await static::timeFill($fill_fn, static::PREFIX, $suffix);
		if ($remote_cache is nonnull) {
			CacheFillTracker::clear();
		}
		$result = $this->convertDataToCacheResult($data, cache_source_t::FILL);

		if ($local_cache is nonnull && ($data is nonnull || ($this::CONFIG['negative_caching'] ?? false))) {
			$local_cache->set($suffix, $data);
			if (self::shouldComputeLocalCacheMetrics()) {
				Metrics::requestCacheCount()->inc(1, shape(
					'op' => 'set',
					'prefix' => $this::PREFIX,
					'result' => 'ok',
					'library' => CacheLibraries::SRC_CACHE,
				));
			}
		}

		/* HAKANA_FIXME[RedundantNonnullTypeComparison] $result is always nonnull */
		if ($result is nonnull && $this->shouldWriteBack($result)) {
			// fill in reverse order from how we fetch since other processes might be hitting the same keys
			if ($remote_cache is nonnull && $should_set_remote) {
				await $remote_cache->setAsync($this::PREFIX, $suffix, $data);
			}

			if ($host_cache is nonnull && $host_cache_key is nonnull) {
				$host_cache->set($host_cache_key, $data, $this->hostCacheTTL());
				if (self::shouldComputeHostCacheMetrics()) {
					Metrics::hostCacheCount()->inc(1, shape(
						'op' => 'set',
						'prefix' => $this::PREFIX,
						'result' => 'ok',
						'library' => CacheLibraries::SRC_CACHE,
					));
				}
			}
		}

		return $result;
	}

	/**
	 * Get from the tier of possible caches, coalescing any misses into a batched fetch.
	 *
	 * Writes back to caches after fill function is called.
	 *
	 * If $expected_type is passed in, it will be verified on fetches from remote or host-level caches.
	 * This does incur a performance cost.
	 */
	final public async function getBatched(
		TSuffix $suffix,
		(function(keyset<TSuffix>): Awaitable<dict<TSuffix, ?TData>>) $batch_fill_fn,
		?typename<TData> $expected_type = null,
	): Awaitable<CacheResult<TData>> {
		$expected_type = $this->selectExpectedDataType($expected_type);

		//
		// If there's no batcher at all or the current batcher is inactive,
		// then create a new one. Otherwise pile on to the existing batch.
		//
		// The main reason to create a new batcher if the existing one is
		// inactive is to support the case where we might have different call
		// paths with different fill functions.
		//
		// Although it's still possible that multiple async branches with different
		// fill functions end up using the same batcher, most likely this is being
		// called in a single callsite with map_async or some other similar function.
		//
		if ($this->fillBatcher is null || !$this->fillBatcher->isActive()) {
			$this->fillBatcher = new AsyncKeyBatcher($batch_fill_fn, static::MAX_FILL_BATCH_SIZE);
		}

		$fill_fn = async (TSuffix $key) ==> {
			invariant($this->fillBatcher is nonnull, "fillBatcher must be defined");
			return await $this->fillBatcher->get($key);
		};

		return await $this->getAsync($suffix, $fill_fn, $expected_type);
	}

	/**
	 * A non-remote implementation of checkAndSetAsync.
	 *
	 * Get from host only, calling the set function if there is a hit.
	 * Writes back to the cache after set function is called. The new value will only be set if the host cache
	 * has not changed. Otherwise we return an error.
	 *
	 * Will throw an error if used where a remote cache is configured, so please use checkAndSetAsync in such
	 * cases instead.
	 *
	 * Because of the nature of APC cas operations, this function is only supported for Caches with TData of int,
	 * we always run type checks and this function will return an error result if it encounters a non-int data value.
	 *
	 * The set function takes the old cache value and returns a new cache value.  This is useful for write through caching.
	 */
	final public async function checkAndSetNonRemoteAsync(
		TSuffix $suffix,
		(function(TData): Awaitable<TData>) $set_fn,
	): Awaitable<CacheResult<TData>> where TData as int {
		$local_cache = $this->localCache;
		$remote_cache = $this->remoteCache;
		$host_cache = $this->hostCache;

		invariant(
			$remote_cache is null,
			'to use checkAndSetNonRemote remote_cache must not be set, please use checkAndSet instead',
		);

		//
		// check host
		//
		invariant($host_cache is nonnull, 'to use checkAndSetNonRemote host_cache must be set');
		$host_cache_key = $this->buildHostCacheKey($this::CONFIG, $this::PREFIX, $suffix);
		$result = $host_cache->get($host_cache_key);

		$data = $result->data();
		if ($result->miss() || $data is null) {
			/* HH_FIXME[4110] unsafe nested mixed */
			return $result;
		}

		if (!($data is int)) {
			return CacheResult::errorResult(
				'currently stored value invalid for a checkAndSet, not an integer',
				cache_source_t::HOST,
				ResultCode::INVALID_ENTRY,
			);
		}

		$new_data = await $set_fn(/* HH_FIXME[4110] unsafe nested mixed */ $data);

		$result = $this->convertDataToCacheResult($new_data, cache_source_t::FILL);

		// If updated value is not eligble to be set to cache we return INVALID_ENTRY;
		if (!$this->shouldWriteBack($result)) {
			return CacheResult::errorResult(
				'value invalid for cache, possibly null',
				cache_source_t::FILL,
				ResultCode::INVALID_ENTRY,
			);
		}

		if ($new_data is int) {
			$success = $host_cache->checkAndSetInt($host_cache_key, $data, $new_data);

			// if cas failed we return EXISTS
			if (!$success) {
				return CacheResult::errorResult(
					'host value changed, can not set',
					cache_source_t::FILL,
					ResultCode::EXISTS,
				);
			}

			if ($local_cache is nonnull && ($new_data is nonnull || ($this::CONFIG['negative_caching'] ?? false))) {
				$local_cache->set($suffix, $new_data);
				if (self::shouldComputeLocalCacheMetrics()) {
					Metrics::requestCacheCount()->inc(
						1,
						shape(
							'op' => 'set',
							'prefix' => $this::PREFIX,
							'result' => 'ok',
							'library' => CacheLibraries::SRC_CACHE,
						),
					);
				}
			}

			return $result;
		} else {
			// If updated value is not eligible to be set to cache we return INVALID_ENTRY
			return CacheResult::errorResult(
				'value invalid for cache, needs to be an integer',
				cache_source_t::FILL,
				ResultCode::INVALID_ENTRY,
			);
		}
	}

	/**
	 * DEPRECATED! Please use a getAsync() with a fill function. checkAndSet does not work with AZ-affine caches and will cause issues.
	 *
	 * Get from remote only, calling the set function if there is a hit.
	 * Writes back to caches after set function is called. The new value will only be set if the remote cache
	 * has not changed. Otherwise we return a miss.
	 *
	 * The set function takes the old cache value and returns a new cache value. This is useful for write through caching.
	 *
	 * If $expected_type is passed in, it will be verified on fetches from remote or host-level caches.
	 * This does incur a performance cost.
	 */
	<<__Deprecated(
		"checkAndSet is deprecated. Please use a getAsync() with a fill function. checkAndSet does not work with AZ-affine caches and will cause issues.",
		0,
	)>>
	final public async function checkAndSetAsync(
		TSuffix $suffix,
		(function(TData): Awaitable<?TData>) $set_fn,
		?typename<TData> $expected_type = null,
	): Awaitable<CacheResult<TData>> {
		$local_cache = $this->localCache;
		$remote_cache = $this->remoteCache;
		$host_cache = $this->hostCache;
		$host_cache_key = null;
		if ($host_cache is nonnull) {
			$host_cache_key = $this->buildHostCacheKey($this::CONFIG, $this::PREFIX, $suffix);
		}

		$expected_type = $this->selectExpectedDataType($expected_type);

		//
		// check remote
		//
		invariant($remote_cache is nonnull, 'to use checkAndSet remote_cache must be set');
		$result = await $remote_cache->getAsync($this::PREFIX, $suffix, $expected_type);


		$data = $result->data();
		if ($result->miss() || $data is null) {
			return $result;
		}

		$new_data = await $set_fn($data);
		if ($new_data is null) {
			return CacheResult::skippedResult();
		}

		$result = $this->convertDataToCacheResult($new_data, cache_source_t::FILL, $result->casToken());

		// If updated value is not eligble to be set to cache we return INVALID_ENTRY;
		if (!$this->shouldWriteBack($result)) {
			return CacheResult::errorResult(
				'value invalid for cache, possibly null',
				cache_source_t::FILL,
				ResultCode::INVALID_ENTRY,
			);
		}

		$set_result = await $remote_cache->setAsync($this::PREFIX, $suffix, $new_data, $result->casToken());
		$success = $set_result === ResultCode::OK;

		// if cas failed we return EXISTS
		if (!$success) {
			return CacheResult::errorResult(
				'remote value changed, can not set',
				cache_source_t::FILL,
				ResultCode::EXISTS,
			);
		}

		if ($host_cache is nonnull && $host_cache_key is nonnull) {
			$host_cache->set($host_cache_key, $new_data, $this->hostCacheTTL());
			if (self::shouldComputeHostCacheMetrics()) {
				Metrics::hostCacheCount()->inc(1, shape(
					'op' => 'set',
					'prefix' => $this::PREFIX,
					'result' => 'ok',
					'library' => CacheLibraries::SRC_CACHE,
				));
			}
		}

		if ($local_cache is nonnull && ($new_data is nonnull || ($this::CONFIG['negative_caching'] ?? false))) {
			$local_cache->set($suffix, $new_data);
			if (self::shouldComputeLocalCacheMetrics()) {
				Metrics::requestCacheCount()->inc(1, shape(
					'op' => 'set',
					'prefix' => $this::PREFIX,
					'result' => 'ok',
					'library' => CacheLibraries::SRC_CACHE,
				));
			}
		}

		return $result;
	}

	/**
	* DEPRECATED! Please use a getAsync() with a fill function. checkAndSet does not work with AZ-affine caches and will cause issues.
	* Check and set with retries. Optionally supports setting a default value if
	* it hasn't been set already.
	*/
	<<__Deprecated(
		"checkAndSet is deprecated. Please use a getAsync() with a fill function. checkAndSet does not work with AZ-affine caches and will cause issues.",
		0,
	)>>
	final public async function checkAndSetWithRetriesAsync(
		TSuffix $suffix,
		(function(TData): Awaitable<?TData>) $set_fn,
		?TData $default_value = null,
		int $remaining_tries = 3,
		?PollingTimer $timer = null,
		?typename<TData> $expected_type = null,
	): Awaitable<CacheResult<TData>> {
		if ($timer is null) {
			$initial_wait_micros = \Duration::fromMilliseconds(100)->inMicroseconds();
			$max_wait_micros = \Duration::fromSeconds(3)->inMicroseconds();
			$timer = new PollingTimer($initial_wait_micros, $max_wait_micros);
		}

		while (!$timer->budgetReached() && $remaining_tries > 0) {
			/* HH_FIXME[4128] */
			$ret = await $this->checkAndSetAsync($suffix, $set_fn, $expected_type);
			$remaining_tries--;

			if ($ret->hit() || $ret->isSkipped()) {
				return $ret;
			}

			if ($default_value is nonnull) {
				$success = await $this->addAsync($suffix, $default_value);
				if ($success) {
					return $this->convertDataToCacheResult($default_value, cache_source_t::FILL);
				}
			}

			await $timer->waitAsync();
		}

		return CacheResult::errorResult('failed to set with retries', \cache_source_t::FILL, ResultCode::EXISTS);
	}

	/**
	 * When we fill a result, should we write it back to other caches?
	 * returns true if it's a hit or negative hit with negative caching enabled
	 */
	private function shouldWriteBack(CacheResult<TData> $result): bool {
		return $result->hit() && ($result->data() is nonnull || ($this::CONFIG['negative_caching'] ?? false));
	}

	<<\Hakana\SecurityAnalysis\IgnorePath()>>
	/**
	 * Set a single value in all caches, overwriting anything that may already be present
	 * If $data is null and the config doesn't allow negative caching, returns false
	 * Otherwise returns true unless a remote cache operation fails (local cache always succeeds)
	 */
	final public async function setAsync(TSuffix $suffix, ?TData $data): Awaitable<bool> {
		$local_cache = $this->localCache;
		$remote_cache = $this->remoteCache;
		$host_cache = $this->hostCache;
		$success = true;

		if ($data is null && !($this::CONFIG['negative_caching'] ?? false)) {
			return false;
		}

		if ($local_cache is nonnull) {
			$local_cache->set($suffix, $data);
			if (self::shouldComputeLocalCacheMetrics()) {
				Metrics::requestCacheCount()->inc(1, shape(
					'op' => 'set',
					'prefix' => $this::PREFIX,
					'result' => 'ok',
					'library' => CacheLibraries::SRC_CACHE,
				));
			}
		}

		if ($host_cache is nonnull) {
			$host_cache_key = $this->buildHostCacheKey($this::CONFIG, $this::PREFIX, $suffix);
			$result = $host_cache->set($host_cache_key, $data, $this->hostCacheTTL());
			if (self::shouldComputeHostCacheMetrics()) {
				Metrics::hostCacheCount()->inc(1, shape(
					'op' => 'set',
					'prefix' => $this::PREFIX,
					'result' => 'ok',
					'library' => CacheLibraries::SRC_CACHE,
				));
			}
			$success = $result;
		}

		if ($remote_cache is nonnull) {
			$result = await $remote_cache->setAsync($this::PREFIX, $suffix, $data);
			$success = $result === ResultCode::OK;
		}
		return $success;
	}

	/*
	 * Get from cache but only looking at the in-memory PerRequestCache.
	 *
	 * Still allows passing in a fill function to populate on miss, similar to how getAsync
	 * works.
	 */
	final public async function getLocalAsync(
		TSuffix $suffix,
		?(function(TSuffix): Awaitable<?TData>) $fill_fn = null,
	): Awaitable<CacheResult<TData>> {
		$local_cache = $this->localCache;

		if ($local_cache is null) {
			return CacheResult::missResult(cache_source_t::LOCAL);
		}

		$result = $local_cache->get($suffix);

		if (self::shouldComputeLocalCacheMetrics()) {
			Metrics::requestCacheCount()->inc(
				1,
				shape(
					'op' => 'get',
					'prefix' => $this::PREFIX,
					'result' => $result->resultLabel(),
					'library' => CacheLibraries::SRC_CACHE,
				),
			);
		}

		if ($result->miss() && $fill_fn) {
			// Don't fetch the same key twice concurrently
			$fill_fn = async (TSuffix $suffix) ==> {
				$this->inProgressFills[$suffix] ??= $fill_fn($suffix);
				$ret = await $this->inProgressFills[$suffix];
				// This is fine even if another call has already unset it
				unset($this->inProgressFills[$suffix]);
				return $ret;
			};

			$data = await $fill_fn($suffix);
			$result = $this->convertDataToCacheResult($data, cache_source_t::FILL);

			if (($data is nonnull || ($this::CONFIG['negative_caching'] ?? false))) {
				$local_cache->set($suffix, $data);
			}
		}

		return $result;
	}

	/**
	 * Set a single value in local cache only, overwriting anything that may already be present
	 * If $data is null and the config doesn't allow negative caching, returns false
	 * Otherwise returns true
	 *
	 * Examples of when this is useful is if you have pre-fetched some data and want the remainder
	 * of the request to be able to take advantage of that pre-fetching.
	 */
	final public function setLocal(TSuffix $suffix, ?TData $data): bool {
		$local_cache = $this->localCache;

		if ($data is null && !($this::CONFIG['negative_caching'] ?? false)) {
			return false;
		}

		if ($local_cache is nonnull) {
			$local_cache->set($suffix, $data);
			if (self::shouldComputeLocalCacheMetrics()) {
				Metrics::requestCacheCount()->inc(1, shape(
					'op' => 'set',
					'prefix' => $this::PREFIX,
					'result' => 'ok',
					'library' => CacheLibraries::SRC_CACHE,
				));
			}
		}

		return true;
	}

	final public function setMultiLocal(KeyedContainer<TSuffix, ?TData> $items): dict<TSuffix, bool> {
		$res = dict<TSuffix, bool>[];
		foreach ($items as $key => $val) {
			$res[$key] = $this->setLocal($key, $val);
		}
		return $res;
	}

	/**
	 * Set a single value in all caches, or return false if something is already present (doesn't overwrite)
	 * If $data is null and the config doesn't allow negative caching, returns false
	 * Otherwise returns true unless a remote cache operation fails (local cache always succeeds)
	 */
	final public async function addAsync(TSuffix $suffix, ?TData $data): Awaitable<bool> {
		$local_cache = $this->localCache;
		$remote_cache = $this->remoteCache;
		$host_cache = $this->hostCache;
		$success = true;

		if ($data is null && !($this::CONFIG['negative_caching'] ?? false)) {
			return false;
		}

		if ($local_cache is nonnull) {
			$local_cache->add($suffix, $data);
			if (self::shouldComputeLocalCacheMetrics()) {
				Metrics::requestCacheCount()->inc(1, shape(
					'op' => 'set',
					'prefix' => $this::PREFIX,
					'result' => 'ok',
					'library' => CacheLibraries::SRC_CACHE,
				));
			}
		}

		if ($host_cache is nonnull) {
			$host_cache_key = $this->buildHostCacheKey($this::CONFIG, $this::PREFIX, $suffix);
			$result = $host_cache->add($host_cache_key, $data, $this->hostCacheTTL());
			if (self::shouldComputeHostCacheMetrics()) {
				Metrics::hostCacheCount()->inc(1, shape(
					'op' => 'add',
					'prefix' => $this::PREFIX,
					'result' => 'ok',
					'library' => CacheLibraries::SRC_CACHE,
				));
			}
			$success = $result;
		}

		if ($remote_cache is nonnull) {
			$result = await $remote_cache->addAsync($this::PREFIX, $suffix, $data);
			$success = $result === ResultCode::OK;
		}
		return $success;
	}

	<<\Hakana\SecurityAnalysis\IgnorePath()>>
	/**
	 * Remove a single value from cache
	 * Returns false if a remote operation failed
	 *
	 * When a remote unset fails, a job is enqueued to retry (see JobHandlerReliableUnset)
	 * This behavior can be controlled by disable_reliable_unset_stream in the cache config.
	 */
	final public async function unsetAsync(TSuffix $suffix): Awaitable<bool> {
		return await $this->unsetImplAsync($suffix, true);
	}

	/**
	 * Removes a single value from the given cache, with no retry.
	 * This function exists only for JobHandlerReliableUnset and is banned for direct use.
	 *
	 * set disable_reliable_unset_stream in the cache config to disable retries for a given prefix.
	 */
	final public static async function unsetNoRetryAsync(
		Cache<TSuffix, TData> $cache,
		TSuffix $suffix,
	): Awaitable<bool> {
		return await $cache->unsetImplAsync($suffix, false);
	}

	private async function unsetImplAsync(TSuffix $suffix, bool $use_reliable_delete_stream): Awaitable<bool> {
		$local_cache = $this->localCache;
		$remote_cache = $this->remoteCache;
		$host_cache = $this->hostCache;
		$success = true;

		if ($local_cache is nonnull) {
			$local_cache->unset($suffix);
			if (self::shouldComputeLocalCacheMetrics()) {
				Metrics::requestCacheCount()->inc(
					1,
					shape(
						'op' => 'unset',
						'prefix' => $this::PREFIX,
						'result' => 'ok',
						'library' => CacheLibraries::SRC_CACHE,
					),
				);
			}
		}

		if ($host_cache is nonnull) {
			$result = $this->buildHostCacheKeysForUnset($this::CONFIG, $this::PREFIX, $suffix)
				|> Dict\map($$, $k ==> $host_cache->unset($k));
			if (self::shouldComputeHostCacheMetrics()) {
				Metrics::hostCacheCount()->inc(
					C\count($result),
					shape(
						'op' => 'unset',
						'prefix' => $this::PREFIX,
						'result' => 'ok',
						'library' => CacheLibraries::SRC_CACHE,
					),
				);
			}
		}

		if ($remote_cache is nonnull) {
			$result = await $remote_cache->unsetAsync($this::PREFIX, $suffix);
			if ($use_reliable_delete_stream) {
				$this->maybeReliablyUnset($suffix, $result);
			}
			$success = ($result === ResultCode::OK || $result === ResultCode::MISS);
		}
		return $success;
	}

	private function maybeReliablyUnset(TSuffix $suffix, ResultCode $result): void {

		if ($this::CONFIG['disable_reliable_unset_stream'] ?? false) {
			Metrics::reliableUnsetStreamDisabledCount()->inc(
				1,
				shape('prefix' => $this::PREFIX, 'library' => CacheLibraries::SRC_CACHE),
			);
			return;
		}

		if ($result === ResultCode::MISS || $result === ResultCode::OK) {
			return;
		}

		if (!\Quota()->memcached_reliable_delete_stream->AllowFailClosed($this::PREFIX)) {
			Metrics::reliableUnsetStreamFRLFailedCount()->inc(
				1,
				shape('prefix' => $this::PREFIX, 'library' => CacheLibraries::SRC_CACHE),
			);
			return;
		}

		$handler = new JobHandlerReliableUnset(shape(
			'cache_classname' => static::class,
			'suffix' => $suffix,
		));

		job_queue_enqueue($handler);
	}

	/**
	 * Get multiple keys from cache
	 * All misses will be filled by one call to the fill_fn, which takes a keyset of missed keys
	 * If $expected_type is passed in, it will be verified on fetches from remote cache and host-level cache only
	 * If $bypass_remote_cache is true, then look at local and host-local cache, but do not get/set from
	 * remote cache, e.g. in cases where the cardinality is too big and it's better to just read from database.
	 */
	<<\Hakana\SecurityAnalysis\IgnorePath>>
	final public async function getMultiAsync(
		Container<TSuffix> $items,
		?(function(keyset<TSuffix>): Awaitable<dict<TSuffix, ?TData>>) $fill_fn = null,
		?typename<TData> $expected_type = null,
		bool $bypass_remote_cache = false,
	): Awaitable<dict<TSuffix, CacheResult<TData>>> {
		$local_cache = $this->localCache;
		$remote_cache = $this->remoteCache;
		$host_cache = $this->hostCache;
		$items = keyset($items);
		$hits = dict[];
		$should_set = true;

		$expected_type = $this->selectExpectedDataType($expected_type);

		if ($local_cache is nonnull) {
			foreach ($items as $suffix) {
				$result = $local_cache->get($suffix);
				if (self::shouldComputeLocalCacheMetrics()) {
					Metrics::requestCacheCount()->inc(
						1,
						shape(
							'op' => 'get',
							'prefix' => $this::PREFIX,
							'result' => $result->resultLabel(),
							'library' => CacheLibraries::SRC_CACHE,
						),
					);
				}
				if ($this->shouldWriteBack($result)) {
					$hits[$suffix] = $result;
					unset($items[$suffix]);
				}
			}
		}

		// return if we hit every key
		if (C\is_empty($items)) {
			/* HH_FIXME[4110] unsafe nested mixed */
			return $hits;
		}

		if ($host_cache is nonnull) {
			foreach ($items as $suffix) {
				$host_cache_key = $this->buildHostCacheKey($this::CONFIG, $this::PREFIX, $suffix);
				$result = $expected_type is null
					? $host_cache->get($host_cache_key)
					: $host_cache->getTyped($host_cache_key, $expected_type);
				if (self::shouldComputeHostCacheMetrics()) {
					Metrics::hostCacheCount()->inc(
						1,
						shape(
							'op' => 'get',
							'prefix' => $this::PREFIX,
							'result' => $result->resultLabel(),
							'library' => CacheLibraries::SRC_CACHE,
						),
					);
				}
				if ($this->shouldWriteBack(/* HH_FIXME[4110] unsafe nested mixed */ $result)) {
					$hits[$suffix] = $result;
					unset($items[$suffix]);
					// set in local
					if ($local_cache is nonnull) {
						$local_cache->set(
							$suffix,
							/* HH_FIXME[4110] unsafe nested mixed */
							$result->data()
						);
						if (self::shouldComputeLocalCacheMetrics()) {
							Metrics::requestCacheCount()->inc(
								1,
								shape(
									'op' => 'set',
									'prefix' => $this::PREFIX,
									'result' => 'ok',
									'library' => CacheLibraries::SRC_CACHE,
								),
							);
						}
					}
				}
			}
		}

		// return if we hit every key
		if (C\is_empty($items)) {
			/* HH_FIXME[4110] unsafe nested mixed */
			return $hits;
		}

		if ($remote_cache is nonnull && !$bypass_remote_cache) {
			$results = await $remote_cache->getMultiAsync($this::PREFIX, $items, $expected_type);
			foreach ($results as $suffix => $result) {
				if ($this->shouldWriteBack($result)) {
					$hits[$suffix] = $result;
					unset($items[$suffix]);
					// set in host cache
					if ($host_cache is nonnull) {
						$host_cache_key = $this->buildHostCacheKey($this::CONFIG, $this::PREFIX, $suffix);
						$host_cache->set($host_cache_key, $result->data(), $this->hostCacheTTL());
						if (self::shouldComputeHostCacheMetrics()) {
							Metrics::hostCacheCount()->inc(
								1,
								shape(
									'op' => 'set',
									'prefix' => $this::PREFIX,
									'result' => 'ok',
									'library' => CacheLibraries::SRC_CACHE,
								),
							);
						}
					}
					// set in local
					if ($local_cache is nonnull) {
						$local_cache->set($suffix, $result->data());
						if (self::shouldComputeLocalCacheMetrics()) {
							Metrics::requestCacheCount()->inc(
								1,
								shape(
									'op' => 'set',
									'prefix' => $this::PREFIX,
									'result' => 'ok',
									'library' => CacheLibraries::SRC_CACHE,
								),
							);
						}
					}
				} else if ($result->isError()) {
					// if we got an error on any cache gets, we don't fill remote cache the rest of the request
					$should_set = false;
				}
			}
		}

		// return if we hit every key
		if (C\is_empty($items)) {
			/* HH_FIXME[4110] unsafe nested mixed */
			return $hits;
		}

		// if no fill function was provided, fill all missed keys with missed results and return
		if ($fill_fn is null) {
			// merge with misses on the left hand side and hits on the right, so that hits are preferred
			/* HH_FIXME[4110] unsafe nested mixed */
			return Dict\fill_keys($items, CacheResult::missResult(cache_source_t::FILL)) |> Dict\merge($$, $hits);
		}

		// We can use config deploy to quickly stop cache from filling a given prefix
		// This is used as a guard rail against sudden load and is meant to be used during incidents
		$short_circuit_config = CacheShortCircuitConfigLoader::loadConfig();
		if ($short_circuit_config->shouldSkipFill(static::PREFIX)) {
			// merge with misses on the left hand side and hits on the right, so that hits are preferred
			/* HH_FIXME[4110] unsafe nested mixed */
			return Dict\fill_keys($items, CacheResult::missResult(cache_source_t::FILL)) |> Dict\merge($$, $hits);
		}

		if ($remote_cache is nonnull) {
			CacheFillTracker::set(static::PREFIX);
		}
		$results = await $fill_fn($items);
		if ($remote_cache is nonnull) {
			CacheFillTracker::clear();
		}

		// log error if we got extra data than what we requested.
		// TODO: later, filter out the extra data from $results.
		$extra_results = Keyset\diff(Keyset\keys($results), $items);
		if (!C\is_empty($extra_results)) {
			await Log::error(
				'cache_get_multi_fill_returned_data_outside_misses',
				dict['extra_keys_count' => C\count($extra_results)],
			);
		}

		// write results back to cache unless we got an error communicating with it earlier
		if ($should_set) {
			// make sure there are null values for any items not present in the results so that negative
			// cache entries are filled properly
			$results_to_set = Dict\fill_keys($items, null) |> Dict\merge($$, $results);

			await $this->setMultiAsync($results_to_set, $bypass_remote_cache);
		} else if ($local_cache is nonnull) {
			// still do local cache even if we're not filling remote cache
			if (!($this::CONFIG['negative_caching'] ?? false)) {
				$results = Dict\filter_nulls($results);
			}
			Dict\map_with_key($results, ($suffix, $data) ==> $local_cache->set($suffix, $data));
			if (self::shouldComputeLocalCacheMetrics()) {
				Metrics::requestCacheCount()->inc(
					C\count($results),
					shape(
						'op' => 'set',
						'prefix' => $this::PREFIX,
						'result' => 'ok',
						'library' => CacheLibraries::SRC_CACHE,
					),
				);
			}
		}

		$hits = $results
			|> Dict\map($$, $r ==> $this->convertDataToCacheResult($r))
			|> Dict\merge($hits, $$);

		/* HH_FIXME[4110] unsafe nested mixed */
		return $hits;
	}

	/**
	* Helper to run the fill function and record the elapsed time.
	* Public because it's also used in RemoteCache.
	*/
	public static async function timeFill<T>(
		(function(TSuffix): Awaitable<?T>) $fill_fn,
		string $prefix,
		TSuffix $suffix,
	): Awaitable<?T> {
		$timer = new \Profiling\ProfilingTimer();
		$start = $timer->start();
		$result = await $fill_fn($suffix);
		$elapsed = $timer->stop($start);
		$elapsed_in_seconds = $elapsed['wait']->inMicroseconds() / 1000000.0;
		Metrics::fillDuration()->observe($elapsed_in_seconds, dict['prefix' => $prefix]);

		// Log slow cache fills. Our assumption is that cache fills will take less than
		// 2 seconds, so starting off with that!
		if ($elapsed_in_seconds > 2.0) {
			await \Logger\Log::event(
				'webapp_slow_cache_fill_duration',
				dict[
					'prefix' => $prefix,
					'elapsed_time_s' => $elapsed_in_seconds,
					'elapsed_time_ms' => $elapsed['wait']->inMilliseconds(),
				],
			);
		}

		return $result;
	}

	/**
	 * Set multiple keys in cache in parallel
	 * Returns a cache result for each individual key
	 */
	final public async function setMultiAsync(
		KeyedContainer<TSuffix, ?TData> $items,
		bool $bypass_remote_cache = false,
	): Awaitable<dict<TSuffix, bool>> {
		$local_cache = $this->localCache;
		$remote_cache = $this->remoteCache;
		$host_cache = $this->hostCache;
		$result = dict[];

		// we only cache nulls if this setting is enabled
		if (!($this::CONFIG['negative_caching'] ?? false)) {
			$items = Dict\filter_nulls($items);
		}

		if ($local_cache is nonnull) {
			foreach ($items as $suffix => $data) {
				$local_cache->set($suffix, $data);
			}
			if (self::shouldComputeLocalCacheMetrics()) {
				Metrics::requestCacheCount()->inc(
					C\count($items),
					shape(
						'op' => 'set',
						'prefix' => $this::PREFIX,
						'result' => 'ok',
						'library' => CacheLibraries::SRC_CACHE,
					),
				);
			}
		}

		if ($host_cache is nonnull) {
			foreach ($items as $suffix => $data) {
				$host_cache_key = $this->buildHostCacheKey($this::CONFIG, $this::PREFIX, $suffix);
				$host_cache->set($host_cache_key, $data, $this->hostCacheTTL());
				if (self::shouldComputeHostCacheMetrics()) {
					Metrics::hostCacheCount()->inc(1, shape(
						'op' => 'set',
						'prefix' => $this::PREFIX,
						'result' => 'ok',
						'library' => CacheLibraries::SRC_CACHE,
					));
				}
			}
		}

		if ($remote_cache is nonnull && !$bypass_remote_cache) {
			$result = Dict\map(
				await $remote_cache->setMultiAsync($this::PREFIX, $items),
				($result_code) ==> {
					return $result_code === ResultCode::OK;
				},
			);
		}
		return $result;
	}

	/**
	 * Remove multiple entries from cache in parallel
	 * Returns a separate result code for each input suffix
	 */
	final public async function unsetMultiAsync(Container<TSuffix> $items): Awaitable<dict<TSuffix, bool>> {
		$local_cache = $this->localCache;
		$remote_cache = $this->remoteCache;
		$host_cache = $this->hostCache;
		$result = dict[];
		$items = keyset($items);

		// we only cache nulls if this setting is enabled
		if (!($this::CONFIG['negative_caching'] ?? false)) {
			$items = Dict\filter_nulls($items);
		}

		if ($local_cache is nonnull) {
			foreach ($items as $suffix => $_data) {
				$local_cache->unset($suffix);
			}
			if (self::shouldComputeLocalCacheMetrics()) {
				Metrics::requestCacheCount()->inc(
					C\count($items),
					shape(
						'op' => 'unset',
						'prefix' => $this::PREFIX,
						'result' => 'ok',
						'library' => CacheLibraries::SRC_CACHE,
					),
				);
			}
		}

		if ($host_cache is nonnull) {
			foreach ($items as $suffix => $_data) {
				$host_cache_key = $this->buildHostCacheKey($this::CONFIG, $this::PREFIX, $suffix);
				$host_cache->unset($host_cache_key);
			}
		}

		if ($remote_cache is nonnull) {
			$result = Dict\map_with_key(
				await $remote_cache->unsetMultiAsync($this::PREFIX, $items),
				($suffix, $result_code) ==> {
					// Re-queue failed unsets in the jobqueue, this is also done for unsetAsync calls
					$this->maybeReliablyUnset($suffix, $result_code);
					return ($result_code === ResultCode::OK || $result_code === ResultCode::MISS);
				},
			);
		}
		return $result;
	}

	/**
	 * For clearing the local cache for this type of cache.
	 * You may want to do this in tests to validate remote cache behaviors.
	 */
	public function clearLocalCache(): void {
		if ($this->localCache is nonnull) {
			$this->localCache->clear();
		}
	}

	/**
	 * For converting raw data from fill functions into cache results
	 * Whether we treat null as a miss or negative hit depends on the config for this cache key
	 */
	private function convertDataToCacheResult(
		?TData $data,
		cache_source_t $source = cache_source_t::FILL,
		?\Cache\CasToken $cas_token = null,
	): CacheResult<TData> {
		if ($data is nonnull) {
			return CacheResult::hitResult($data, $source, $cas_token);
		}

		if ($this::CONFIG['negative_caching'] ?? false) {
			return CacheResult::negativeResult($source);
		}

		return CacheResult::missResult($source);
	}

	/**
	 * Returns the datatype expected in the cache for this prefix and, if
	 * non-null, enables validation by default on all fetches. Even if this
	 * method returns null, type validation can be enabled on specific fetches
	 * by passing $expected_type to those specific calls.
	 *
	 * Subclasses should override this if they want additional type safety.
	 *
	 * Caveats:
	 *  - Returning non-null does cause a performance penalty on fetches.
	 *  - if you change the type at some point in the future, you must
	 *    update the `schema_versions` and `current_schema_version`.
	 *  - if you currently use multiple different datatypes within the same
	 *    prefix (not recommended), you cannot enable class-level type
	 *    validation.
	 */
	protected function expectedDataType(): ?typename<TData> {
		return null;
	}

	/**
	 * Finds the proper expected type, and emits a metric for tracking how
	 * often a type is provided.
	 */
	private function selectExpectedDataType(?typename<TData> $expected_type): ?typename<TData> {
		$specified_where = CacheExpectedTypeSource::UNSPECIFIED;

		if ($expected_type is nonnull) {
			$specified_where = CacheExpectedTypeSource::CALL_SITE;
		} else {
			$expected_type = $this->expectedDataType();
			if ($expected_type is nonnull) {
				$specified_where = CacheExpectedTypeSource::CLASS_FUNC;
			}
		}

		Metrics::expectedTypeCount()->inc(
			1,
			shape(
				'prefix' => $this::PREFIX,
				'library' => CacheLibraries::SRC_CACHE,
				'specified_at' => $specified_where,
				'type_provided' => $expected_type is nonnull,
			),
		);

		return $expected_type;
	}

	/*
	 * Build the full cache key to be used for a given prefix/suffix combo based on config.
	 * This should only be used for keys passed to the per-host cache, as the remote cache
	 * code needs keys that include AZ routing information.
	 */
	protected function buildHostCacheKey<T as arraykey>(cache_config_t<T> $config, string $prefix, T $suffix): string {
		$key = CacheKey::newHostKey($prefix, $suffix, $config);
		return $key->getString();
	}

	/**
	 * For unsetting, we need to unset all known schema version keys this builds each of them.
	 * This should only be used for keys passed to the per-host cache, as the remote cache
	 * code needs keys that include AZ routing information.
	 */
	private function buildHostCacheKeysForUnset<T as arraykey>(
		cache_config_t<T> $config,
		string $prefix,
		T $suffix,
	): keyset<string> {
		$key = CacheKey::newHostKey($prefix, $suffix, $config);
		return Keyset\map($key->allVersions(), $key ==> $key->getString());
	}

	/*
	* Helper to get the TTL for the host cache with some fuzzing.
	*/
	private function hostCacheTTL(): int {
		$ttl = $this::CONFIG['host_cache_ttl'] ?? -1;
		if ($ttl === -1) return $ttl;
		return \util_fuzz($ttl, 0.1);
	}
}
