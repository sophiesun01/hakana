// // simplest AST 

// foo();

// // let's add complexity, function call + assignments

// $a = 1;
// $b = "hello";
// foo($a, $b);


// // function def

// $a = 1;
// $b = "hello";

// foo($a, $b);

// function foo(int $a, int $b) : void {

// }

// // function with statements
// $a = 1;
// $b = 2;
// foo($a, $b);

// function foo(int $a, int $b) : int {
// 	$c = 1.0;
// 	$d = 0;
// 	return $c;
// }

// // function with if statements
// $a = 1;
// $b = "hello";

// foo($a, $b);

// function foo(int $a, int $b) : arraykey {
// 	$c = 1.0;
// 	$d = 0;
	
// 	if ($b == "hello") {
// 		return $b;
// 	}
	
// 	return $c;
// }

// // function with switch statements
// $a = 1;
// $b = "hello";

// foo($a, $b);

// function foo(int $a, int $b) : float {
// 	$c = 1.0;
// 	$d = 0;
	
// 	switch ($b) {
// 		case "hello":
// 			return "yes";
// 		default:
// 			return "no";
// 	}
	
// 	return $c;
// }

// function with foreach statements
$a = vec[1,2,3];

foo($a, $b);

function foo(vec<int> $a) : arraykey {
	$c = 1.0;
	$d = 0;
	
	foreach ($a as $val) {
		
	}
	
	return $c;
}