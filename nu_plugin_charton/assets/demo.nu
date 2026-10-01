# Generate the synthetic dataset used by the demo GIF.
# Run from this directory:  nu demo.nu
let pi = 3.141592653589793
let data = (1..3 | each {|k|
  1..120 | each {|i|
    let a = ($i / 120 * $pi * 2)
    let r = (1.0 + $k * 1.7 + (($i | math sin) * 0.6))
    {
      x: ($r * ($a | math cos) | math round --precision 4)
      y: ($r * ($a | math sin) | math round --precision 4)
      size: (1.0 + ($i mod 6))
      group: $"cluster ($k)"
    }
  }
} | flatten)
$data | to csv | save -f demo.csv
print $"wrote (($data | length)) rows to demo.csv"
