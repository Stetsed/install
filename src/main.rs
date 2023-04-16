mod zfs;
mod chroot;

use std::io::{self, Write};
use std::env;
use std::process::Command;
use std::fs;
use std::thread;
use std::time::Duration;

use zfs::*;
use chroot::*;

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
                "--user" => user(),
                _ => print!("Invalid Flag Passed"),
            }
        }
    } else {
        // If no arguments are provided, call the no_flag_passed function to handle user input.
        no_flag_passed();
    }
}

// Function to prompt the user for input when no flag is provided.
fn no_flag_passed(){
    println!("Choose an option:");
    println!("1. ZFS");
    println!("2. Chroot");
    println!("3. User");

    // Read user input and call the appropriate function based on the user's choice.
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).expect("Failed to read line");

    match choice.trim() {
        "1" => zfs::zfs(),
        "2" => chroot::chroot(),
        "3" => user(),
        _ => println!("Invalid choice"),
    }
}

fn execute_command(command: &str) -> std::io::Result<()> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        eprintln!("Command '{}' failed with exit status: {:?}", command, output.status);
        std::process::exit(1);
    }

    println!("{}", String::from_utf8_lossy(&output.stdout));

    thread::sleep(Duration::from_secs(3));

    Ok(())
}

// This function executes a series of shell commands to install packages and perform other setup tasks for the user.
fn user() {
    // Create the user's home directory
    user_create_home();

    // Install Yay packages
    user_yay_packages();

    // Install dotfiles
    user_install_dotfiles();

    // Install additional packages
    user_extras();

    // Ask the user if they want to use Stetsed's dotfiles and install them if they say yes
    let mut input = String::new();
    print!("Do you want to use Stetsed's Home Configuration(Say no if your not Stetsed)? (y/n): ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();

    if input.trim().eq_ignore_ascii_case("y") {
        user_extras_stetsed();
    }
    
    // Print a thank-you message and exit the program
    print!("Thank you for using Stetsed's Installer! Hope it helped you! :");
    std::process::exit(0);
}

// This function creates the home directory for the current user by executing a series of shell commands.
fn user_create_home() -> std::io::Result<String>  {
    // Get the current user's name
    let output = Command::new("whoami")
        .output()
        .expect("failed to execute process");
    let whoami_output = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Create the user's home directory and set the owner and permissions
    let commands = vec![
        format!("sudo mkdir /home/{}", whoami_output),
        format!("sudo chown {}:{} -R /home/{}", whoami_output, whoami_output, whoami_output),
        format!("sudo chmod 700 /home/{}", whoami_output),
    ];

    // Execute the commands in the vector
    for command in commands {
        execute_command(&command);
    }

    // Return a `String` indicating the completion of the operation
    Ok(format!("Home Created"))
}

fn user_yay_packages() -> std::io::Result<String>  {
    // Get the username of the current user
    let output = Command::new("whoami")
        .output()
        .expect("failed to execute process");

    let whoami_output = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let commands = vec![
        // Clone the yay package manager from AUR
        format!("cd /home/{} && git clone https://aur.archlinux.org/yay-bin.git", whoami_output),
        // Build and install the yay package manager
        format!("cd /home/{} && cd yay-bin && makepkg -s && sudo pacman -U --noconfirm yay-bin* && cd .. && rm -rf yay-bin", whoami_output),
        // Install packages using yay
        "yay -Syu --noconfirm --answerdiff=None imagemagick kitty ripgrep unzip bat pavucontrol pipewire-pulse dunst bluedevil bluez-utils brightnessctl grimblast-git neovim network-manager-applet rofi-lbonn-wayland-git starship thunar thunar-archive-plugin thunar-volman  webcord-bin wl-clipboard librewolf-bin neofetch swaybg waybar-hyprland-git btop tldr swaylock-effects obsidian fish hyprland npm xdg-desktop-portal-hyprland-git exa noto-fonts-emoji qt5-wayland qt6-wayland blueman swappy playerctl wlogout sddm-git nano ttf-jetbrains-mono-nerd lazygit swayidle".to_string(),
    ];

    for command in commands {
        execute_command(&command);
    }
    Ok(format!("Yay and Packages Installed"))
}

// Function to install dotfiles from a user's repository or Stetsed's Dotfiles
fn user_install_dotfiles() -> std::io::Result<String> {

    // Initialize variables to hold the URLs of the dotfiles repositories
    let mut dotfiles_url = String::new(); 
    let mut ssh_url = String::new(); 

    // Ask the user if they want to use Stetsed's Dotfiles or their own
    let mut input = String::new(); 
    print!("Do you want to use Stetsed's Dotfiles? (y/n): "); 
    io::stdout().flush().unwrap(); 
    io::stdin().read_line(&mut input).unwrap(); 

    // If the user wants to use Stetsed's Dotfiles, set the URLs accordingly
    if input.trim().eq_ignore_ascii_case("y") {
        dotfiles_url = "https://github.com/Stetsed/.dotfiles.git".to_owned(); 
        ssh_url = "git@github.com:Stetsed/.dotfiles.git".to_owned(); 
    } else {
        // Ask the user for their github dotfiles repository and set the URLs accordingly
        print!("Enter your github dotfiles repository (username/repository_name): "); 
        let mut dotfiles_repo = String::new(); 
        io::stdout().flush().unwrap(); 
        io::stdin().read_line(&mut dotfiles_repo).unwrap(); 
        let dotfiles_repo_trim = dotfiles_repo.trim().to_owned(); 
        dotfiles_url = format!("https://github.com/{}.git", dotfiles_repo_trim); 
        ssh_url = format!("git@github.com:{}.git", dotfiles_repo_trim); 
    }

    // Create a vector of commands to execute
    let commands = vec![
        format!("git clone --bare {} $HOME/.dotfiles", dotfiles_url), // Clone the dotfiles repository to ~/.dotfiles
        "/usr/bin/git --git-dir=$HOME/.dotfiles/ --work-tree=$HOME checkout -f".to_string(), // Checkout the dotfiles to the home directory
        "/usr/bin/git --git-dir=$HOME/.dotfiles/ --work-tree=$HOME config status.showUntrackedFiles no".to_string(), // Ignore untracked files in the dotfiles directory
        format!("/usr/bin/git --git-dir=$HOME/.dotfiles/ --work-tree=$HOME config remote set-url origin {}", ssh_url), // Set the origin URL of the dotfiles repository
    ];

    // Loop through the commands and execute each one
    for command in commands {
        execute_command(&command);
    }

    Ok(format!("Dotfiles Installed")) // Return a success message
}

// Function to enable and configure various system services
fn user_extras() -> std::io::Result<String> {

    // Create a vector of commands to execute
    let commands = vec![
        "sudo systemctl enable --now bluetooth && sudo systemctl enable sddm", // Enable and start Bluetooth and SDDM services
        "systemctl --user enable --now pipewire", // Enable and start the user-level pipewire service
        "systemctl enable --now pipewire-pulse", // Enable and start the pipewire-pulse service
        "sudo timedatectl set-ntp true && sudo timedatectl set-timezone Europe/Amsterdam", // Set the timezone and enable NTP synchronization
    ];

    // Loop through the commands and execute each one
    for command in commands {
        execute_command(&command);
    }

    Ok(format!("Extra's Done")) // Return a success message
}

// Function to enable and configure various system services specific to Stetsed's setup
fn user_extras_stetsed() -> std::io::Result<String> {

    // Create a vector of commands to execute
    let commands = vec![
        "echo '10.4.78.251:/mnt/Vault/Storage /mnt/data nfs defaults,_netdev,x-systemd.automount,x-systemd.mount-timeout=10,noauto 0 0' | sudo tee -a /etc/fstab", // Add an NFS mount to /etc/fstab
        "sudo mkdir /mnt/data", // Create a mount point directory
        "sudo mount -t nfs 10.4.78.251:/mnt/Vault/Storage /mnt/data", // Mount the NFS share to the directory
        "ln -s /mnt/data/Stetsed/Storage ~/Storage", // Create a symlink for Stetsed's Storage directory
        "ln -s /mnt/data/Stetsed/Documents ~/Documents", // Create a symlink for Stetsed's Documents directory
        "echo -e '[Autologin]\nUser=stetsed\nSession=hyprland' | sudo tee -a /etc/sddm.conf", // Configure SDDM to autologin as user 'stetsed' and use the 'hyprland' session
        "sudo groupadd autologin && sudo usermod -aG autologin stetsed", // Add the user 'stetsed' to the 'autologin' group
    ];

    // Loop through the commands and execute each one
    for command in commands {
        execute_command(&command);
    }
    Ok(format!("Stetsed Extra's Done")) // Return a success message
}

