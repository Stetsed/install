use std::io::{self, Write};
use std::env;
use std::process::Command;
use std::fs;
use std::thread;
use std::time::Duration;

pub fn chroot() {
    // Prompt the user for a username and password
    print!("Enter username: ");
    io::stdout().flush();
    let mut username = String::new();
    io::stdin().read_line(&mut username);

    print!("Enter password: ");
    io::stdout().flush();
    let mut password = String::new();
    io::stdin().read_line(&mut password);

    print!("Enter your Platform in Lower Case(intel/amd): ");
    io::stdout().flush();
    let mut platform = String::new();
    io::stdin().read_line(&mut platform);

    // Call the chroot_install() function to install packages and configure the ZFS filesystem
    chroot_install(username.trim(), password.trim(), platform.trim());

    // Exit the program
    std::process::exit(0);
}

// This function installs packages and configures the ZFS filesystem in a chroot environment by executing a sequence of shell commands using `Command` from the standard library. The commands add a repository, install packages, create a user, set up a cache file, configure the bootloader, enable services, and generate an initramfs. The function takes a username and password as input and returns a `String` indicating the completion of the operation.

pub fn chroot_install(username: &str, password: &str, platform: &str) -> std::io::Result<String> {
    // Define a vector of shell commands to execute
    let commands = vec![
        "echo -e '[archzfs]\nServer = https://archzfs.com/$repo/$arch' >>/etc/pacman.conf".to_string(), // Add a repository
        "pacman-key -r DDF7DB817396A49B2A2723F7403BD972F75D9D76".to_string(), // Add a repository key
        "pacman-key --lsign-key DDF7DB817396A49B2A2723F7403BD972F75D9D76".to_string(), // Sign the repository key
        "pacman -Syu --noconfirm".to_string(), // Update the system
        format!("pacman -S --noconfirm nfs-utils linux-headers zfs-dkms openssh networkmanager fish git {}-ucode", &platform), // Install packages
        format!("useradd -m -G wheel -s /usr/bin/fish {}", username), // Create a user
        format!("(echo '{}'; echo '{}') | passwd {}", password, password, username), // Set the user password
        "zpool set cachefile=/etc/zfs/zpool.cache zroot".to_string(), // Set up the cache file
        "bootctl install".to_string(), // Install the bootloader
        "systemctl enable NetworkManager".to_string(), // Enable NetworkManager service
        format!("echo -e 'title Arch Linux\nlinux vmlinuz-linux\ninitrd {}-ucode.img\ninitrd initramfs-linux.img\noptions zfs=zroot/ROOT/default rw' > /boot/loader/entries/arch.conf", &platform), // Configure the bootloader
        "echo 'default arch' >> /boot/loader/loader.conf".to_string(), // Configure the bootloader
        "echo '%wheel ALL=(ALL:ALL) ALL' >> /etc/sudoers".to_string(), // Allow wheel group to execute sudo
        "systemctl enable zfs-scrub-weekly@zroot.timer".to_string(), // Enable ZFS scrub timer
        "systemctl enable zfs.target".to_string(), // Enable ZFS target
        "systemctl enable zfs-import-cache".to_string(),
        "systemctl enable zfs-mount".to_string(), // Enable ZFS mount
        "zgenhostid $(hostid)".to_string(), // Generate hostid for the system
        "sed -i 's/keyboard keymap/keyboard zfs keymap/g' /etc/mkinitcpio.conf".to_string(), // Configure keyboard and keymap for initramfs
        "mkinitcpio -P".to_string(), // Generate initramfs
    ];

    // Execute the commands in the vector
    for command in commands {
        execute_command(&command);
    }

    // Return a `String` indicating the completion of the operation
    Ok(format!("Chroot Install Done"))
}

pub fn execute_command(command: &str) -> std::io::Result<()> {
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
