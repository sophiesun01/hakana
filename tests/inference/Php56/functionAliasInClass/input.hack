namespace Name\Space {
    function f(): void { echo __FUNCTION__."\n"; }
}

namespace Noom\Spice {
    use function Name\Space\f;

    final class A {
        public function fooFoo(): void {
            f();
            \Name\Space\f();
        }
    }
}