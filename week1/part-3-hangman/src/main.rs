use std::io;
use std::io::Write;

fn get_a_word() -> String {
    let str: String = String::from("mamemamehong");
    return str;
}

fn init_a_guess_word(n : usize) -> String {
    let mut str : String = String::new();
    for _i in 0..n {
        str.push('-');
    }
    return str;
}

fn replace_i_word(word: String, i : usize, chr : char) -> String {
    let mut x : usize = 0;
    let mut new_word : String = String::new();
    for c in word.chars() {
        if x == i {
            new_word.push(chr);
        }else{
            new_word.push(c);
        }
        x += 1;
    }

    return new_word;
}

fn main() {
    let w: String = get_a_word();
    let mut guess_word : String = init_a_guess_word(w.len());
    let mut cnt : i32 = 5;
    let mut guessed_letter : String = String::new();
    let n : usize = w.len();
    while cnt != 0 {
        println!("The word so far is {}", guess_word);
        println!("You have guessed the following letters: {}", guessed_letter);
        println!("You have {} guesses left", cnt);

        print!("Please guess a letter: ");


        io::stdout()
            .flush()
            .expect("Error flushing stdout.");
        let mut guess : String = String::new();
        io::stdin()
            .read_line(&mut guess)
            .expect("Error reading line.");

        guess = guess.replace("\n", "");
        guessed_letter = guessed_letter + &guess;

        let mut flag : bool = false;
        for i in 0..n {
            if w.chars().nth(i) == guess.chars().nth(0) {
                flag = true;
                guess_word = replace_i_word(guess_word, i, w.chars().nth(i).unwrap());
            }
        }

        println!("");

        if !flag {
            cnt -= 1;
        }else if guess_word == w {
            print!("Success! The word is {}", guess_word);
            break;
        }

        if cnt == 0 {
            print!("failed! game over.");
            break;
        }
    }
}
