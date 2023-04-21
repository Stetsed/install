mod chroot;
mod user;
mod zfs;

use std::env;
use std::io;

fn main() {
    // Collect command line arguments into a vector of strings.
    let args: Vec<String> = env::args().collect();

    // If any arguments are provided, iterate through them, skipping the first one (the executable name).
    if args.len() > 1 {
        for arg in args.iter().skip(1) {
            // Call the appropriate function based on the argument provided.
            match arg.as_str() {
                "--zfs" => zfs::zfs(),
                "--chroot" => chroot::chroot(),
                "--user" => user::user(),
                _ => print!("Invalid Flag Passed"),
            }
        }
    } else {
        // If no arguments are provided, call the no_flag_passed function to handle user input.
        no_flag_passed();
    }
}

// Function to prompt the user for input when no flag is provided.
fn no_flag_passed() {
    println!("Choose an option:");
    println!("1. ZFS");
    println!("2. Chroot");
    println!("3. User");

    // Read user input and call the appropriate function based on the user's choice.
    let mut choice = String::new();
    io::stdin()
        .read_line(&mut choice)
        .expect("Failed to read line");

    match choice.trim() {
        "1" => zfs::zfs(),
        "2" => chroot::chroot(),
        "3" => user::user(),
        _ => println!("Invalid choice"),
    }
}
