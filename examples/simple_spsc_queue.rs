use lockfree::spsc_queue;

use std::thread;

fn main() {
    let (tx, rx) = spsc_queue::new_queue(500);

    thread::spawn(move|| {
        for i in 0..100000 {
            loop {
                match tx.try_push(i) {
                    None => break,
                    Some(_) => {},
                }
            }
            //println!("Wrote {}", i);
        }
    });

    for i in 0..100000 {
        loop {
            match rx.try_pop() {
                None => {}
                Some(v) => { assert!(v == i, "v: {} != i: {}", v, i); break; }
            }
        }
    }
}
