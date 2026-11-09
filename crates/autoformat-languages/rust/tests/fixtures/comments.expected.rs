//! Crate level documentation.

/// Documentation for a constant.
pub const LIMIT: usize = 10;         // the limit
pub const OTHER_LIMIT: usize = 200;  // another limit
pub const X: u8 = 1;                 /* block after code */

fn compute(a: i32,  // first
    b: i32) -> i32 {
    /**
     * Old style doc comment
     * with misaligned stars
     */
    let sum = a + b;      // add them
    let product = a * b;  // multiply

    // Final result.
    sum + product
    // trailing comment before closing brace
}

/*
  Free text block comment
    with its own indentation
*/
fn empty() {}
