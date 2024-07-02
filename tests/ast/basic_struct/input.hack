// Define a class
class Point {
  public float $x;
  public float $y;

  // Constructor

  public function __construct(float $x, float $y) {
    $this->x = $x;
    $this->y = $y;
  }

  // // Method to print the point
  public function print(): void {
    echo "Point: (" . $this->x . ", " . $this->y . ")\n";
  }
}

<<__EntryPoint>>
function main(): void {
  // Create an instance of the class
  $point = new Point(1.0, 2.0);
  
  // Print the point
  $point->print();
}