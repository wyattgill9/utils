mod math;
use math::fib; 
use std::io;

fn main() {
    let mut input = String::new();

    io::stdin().read_line(&mut input).expect("Failed to read line");

    let number: i32 = input.trim().parse().expect("Please enter a valid number");

    let result = fib::fib(number.try_into().unwrap());
    println!("{:?}", result);
}