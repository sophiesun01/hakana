final class A {
    const MAP = dict[
        "a" => 1,
        "b" => 2,
        "c" => 3,
        "d" => 4,
        "e" => 5,
        "f" => 6,
        "g" => 7,
        "h" => 8,
        "i" => 9,
        "j" => 10,
        "k" => 11,
    ];

    public function doWork(string $a): void {
        if (!array_key_exists($a, self::MAP)) {}
    }
}