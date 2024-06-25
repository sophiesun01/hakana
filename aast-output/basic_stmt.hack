// $test = 1;
// if ($test){};

Type Foo = shape(
    'id' => int,
    'username' => string,
    ?'latitude' => float,
    ?'longitude' => float,
);
// $hello = 0;
// type Foo = shape('bar' => ?string);
// $x = shape('bar' => 'baz'); // valid
// $x = shape('bar' => null); // valid
$x = shape('id'=>0,
		'username' => 'Sophie',
		'latitude' => 1.98);

echo $x;