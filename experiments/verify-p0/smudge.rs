// Same-length byte-rewriting smudge filter used by the P0 adversarial fixtures.
// It upper-cases ASCII letters, so `alpha\n` becomes `ALPHA\n` (same 6 bytes).
use std::io::{Read, Write};

fn main() {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).expect("stdin");
    for b in buf.iter_mut() {
        if b.is_ascii_lowercase() {
            *b = b.to_ascii_uppercase();
        }
    }
    std::io::stdout().write_all(&buf).expect("stdout");
}
