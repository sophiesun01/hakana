abstract class A {}
final class B extends A {}
final class C extends A {}

function takesA(A $a): void {}

function foo(?A $a, ?A $b): void {
    if (($a is B && $b is B)
        || ($a is C && $b is C)
    ) {
        takesA($a);
        takesA($b);
    }
}