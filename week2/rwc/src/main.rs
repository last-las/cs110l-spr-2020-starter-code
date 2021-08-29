use std::{env, io};
use std::process;
use std::fs::File;
use std::io::BufRead;

fn read_file_lines(filename: &String) -> Result<Vec<String>, io::Error> {
    let file: File = File::open(filename)?;

    let mut file_lines: Vec<String> = Vec::new();

    for line in io::BufReader::new(file).lines() {
        let mut line_str: String = line?;
        file_lines.push(line_str);
    }

    return Ok(file_lines);
}

fn read_words_cnt(file_lines: &Vec<String>) -> usize {
    let mut cnt: usize = 0;
    for file_line in file_lines.iter() {
        cnt = cnt + file_line.split(" ").count();
    }
    return cnt;
}

fn read_lines_cnt(file_lines: &Vec<String>) -> usize {
    file_lines.len()
}

fn read_letters_cnt(file_lines: &Vec<String>) -> usize {
    let mut cnt: usize = 0;
    for file_line in file_lines.iter() {
        for i in 0..file_line.len() {
            let b: u8 = file_line.as_bytes()[i];
            if 97 <= b && b <= 122 {
                cnt += 1;
            } else if 65 <= b && b <= 90 {
                cnt += 1;
            }
        }
    }
    return cnt;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename = &args[1];
    // Your code here :)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_words_cnt() {
        let lines_result = read_file_lines(&String::from("test.txt"));
        assert!(lines_result.is_ok());
        let file_lines = lines_result.unwrap();
        let words_cnt: usize = read_words_cnt(&file_lines);
        assert_eq!(words_cnt, 20);
    }

    #[test]
    fn test_read_lines_cnt() {
        let lines_result = read_file_lines(&String::from("test.txt"));
        assert!(lines_result.is_ok());
        let file_lines = lines_result.unwrap();
        let lines_cnt: usize = read_lines_cnt(&file_lines);
        assert_eq!(lines_cnt, 3);
    }

    #[test]
    fn test_read_letters_cnt() {
        let lines_result = read_file_lines(&String::from("test.txt"));
        assert!(lines_result.is_ok());
        let file_lines = lines_result.unwrap();
        let letters_cnt : usize = read_letters_cnt(&file_lines);
        assert_eq!(letters_cnt, 63);
    }
}