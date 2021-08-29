use crossbeam_channel;
use std::{thread, time};
use std::result::Result::Ok;
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
    where
        F: FnOnce(T) -> U + Send + Copy + 'static,
        T: Send + 'static,
        U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    // TODO: implement parallel map!
    let (t_sender, t_receiver): (Sender<(usize, T)>, Receiver<(usize, T)>) = crossbeam_channel::unbounded();
    let (u_sender, u_receiver): (Sender<(usize, U)>, Receiver<(usize, U)>) = crossbeam_channel::unbounded();
    let mut threads = Vec::new();
    for _ in 0..num_threads {
        let t_receiver = t_receiver.clone();
        let u_sender = u_sender.clone();
        threads.push(thread::spawn(move || {
            while let Ok(t_tuple) = t_receiver.recv() {
                let i = t_tuple.0;
                let u = f(t_tuple.1);

                u_sender.send((i, u));
            }
        }));
    }

    let n = input_vec.len();
    let mut i: usize = 0;
    for t in input_vec {
        t_sender.send((i, t));
        i += 1;
    }

    drop(t_sender);

    for _ in 0..n {
        output_vec.push(U::default());
    }

    for _ in 0..n {
        if let Ok(u_tuple) = u_receiver.recv() {
            output_vec[u_tuple.0] = u_tuple.1;
        }
    }

    for thread in threads {
        thread.join();
    }

    return output_vec;
}

fn main() {}

mod test {
    use super::*;

    use reqwest;
    use select;
    /*#[macro_use]
    extern crate error_chain;*/

    use std::sync::{Arc, Mutex};
    use std::{thread};
    use select::document::Document;
    use select::predicate::Name;

    /*error_chain! {
        foreign_links {
            ReqError(reqwest::Error);
            IoError(std::io::Error);
        }
    }*/


    #[test]
    fn test_parallel_map() {
        let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
        let squares = parallel_map(v, 10, |num| {
            println!("{} squared is {}", num, num * num);
            thread::sleep(time::Duration::from_millis(500));
            num * num
        });
        println!("squares: {:?}", squares);
        assert_eq!(squares, vec![36, 49, 64, 81, 100, 1, 4, 9, 16, 25, 144, 324, 121, 25, 400]);
    }

    #[test]
    fn link_explorer_with_parallel_map() -> Result<()> {
        let url = "https://en.wikipedia.org/wiki/Multithreading_(computer_architecture)";
        let body = reqwest::blocking::get(url)?.text()?;
        Ok(())
    }
}
