async function async_fn(string $s): Awaitable<string> {
    return $s;
}

function target_fn(string $s): string {
    echo 'async from sync';
    return HH\Asio\join(async_fn($s));
}

function sync_fn_calls_sync(string $s): string {
    echo 'sync from sync';
    return target_fn($s);
}

async function async_fn_calls_sync(string $s): Awaitable<string> {
    echo 'sync from sync';
    return target_fn($s);
}