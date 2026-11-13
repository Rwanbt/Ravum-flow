// Pass-through filter used as the `clean` side of the fixture driver, so Git's
// own index refresh does not fail when only a smudge filter is configured.
use std::io::{Read, Write};

fn main() {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).expect("stdin");
    std::io::stdout().write_all(&buf).expect("stdout");
}
