final class C {
    public function foo() : ?string {
        return null;
    }
}

function foo(C $c) : void {
    new DateTime($c->foo() ?? "");
}