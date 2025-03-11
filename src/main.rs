mod lfs;
use lfs::stack::LockFreeStack; 

fn main() {
    let stack = LockFreeStack::new();
    stack.push(42);
    stack.pop();
}