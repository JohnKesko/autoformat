#![allow(dead_code)]
use std::fmt::{self,Display};
pub(crate) struct Wrapper<'a,T:Display+?Sized>{inner:&'a T,count:usize}
impl<'a,T:Display+?Sized> Display for Wrapper<'a,T>{
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{
        write!(f,"{}x{}",self.inner,self.count)
    }
}
macro_rules! square {
    ($x:expr) => { $x * $x };
    ($($x:expr),*) => { vec![$($x * $x),*] };
}
fn generic<T>(items:Vec<T>)->Option<T> where T:Clone+Default, {
    let first=items.first().cloned();
    let closure=move||first.clone();
    let x: &[u8] = &[1,2,3];
    let y = *x.get(0).unwrap_or(&0) as u32;
    let r = 1..=10;
    let s = "short";
    let t = 'c';
    let u = b"bytes";
    let label = 'outer: loop { break 'outer 5; };
    let very_long_string_that_cannot_be_wrapped = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let nested = outer_function(inner_function(argument_number_one, argument_number_two), another_inner(argument_three, argument_four), fifth);
    println!("value: {} {} {} {} {}", first_value_with_long_name, second_value_with_long_name, third_value_name, fourth, fifth_value);
    if let Some(v) = items.get(0) { return Some(v.clone()); }
    let _ = (closure, x, y, r, s, t, u, label, very_long_string_that_cannot_be_wrapped, nested);
    // standalone comment
    items.into_iter()
        .filter(|item| {
            // inside closure
            true
        })
        .next()
}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() { assert_eq!(2+2, 4); }
}
