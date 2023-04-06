use std::io::{self, Write};
use std::env;
use std::process::Command;
use std::fs;
use std::thread;
use std::time::Duration;

fn main() {
    // Collect command line arguments into a vector of strings.
    let args: Vec<String> = env::args().collect();

    // If any arguments are provided, iterate through them, skipping the first one (the executable name).
    if args.len() > 1 {
        for arg in args.iter().skip(1) {
            // Call the appropriate function based on the argument provided.
            match arg.as_str() {
                "--zfs" => zfs(),
                "--chroot" => chroot(),
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
        "1" => zfs(),
        "2" => chroot(),
        "3" => user(),
        _ => println!("Invalid choice"),
    }
}

// Function to setup ZFS and call the necessary sub-functions.
fn zfs() {
    // Call the necessary sub-functions in the correct order.
    zfs_get_zfs();
    let selected_drive = zfs_select_drive().unwrap_or_else(|err| {
        eprintln!("Failed to select drive: {}", err);
        String::new()
    });
    zfs_partition_drive(&selected_drive);
    zfs_setup_filesystem(&selected_drive);
    zfs_setup_basesystem();

    // Exit the program with a successful status code.
    std::process::exit(0);
}

// Function to download and install ZFS.
fn zfs_get_zfs() -> std::io::Result<String> {
    // Execute the curl command to download the ZFS installation script.
    let output = Command::new("curl")
        .args(&["-s", "https://raw.githubusercontent.com/eoli3n/archiso-zfs/master/init"])
        .output()
        .expect("failed to execute curl command");

    // Convert the output to a string and execute the downloaded script using the bash command.
    let output_str = String::from_utf8_lossy(&output.stdout);
    Command::new("bash")
        .arg("-c")
        .arg(output_str.trim())
        .status()
        .expect("failed to execute bash command");

    // Return a success message.
    Ok(format!("Installed ZFS"))
}

// This function lists all available drives in the /dev/disk/by-id directory, and prompts the user to select one. It returns the selected drive as a String.
fn zfs_select_drive() -> Result<String, std::io::Error> {
    let devices_dir = "/dev/disk/by-id";

    // Get a list of devices in the directory
    let entries = fs::read_dir(devices_dir)?;

    let mut devices = Vec::new();
    // Iterate through the directory entries and collect device names that do not contain "part"
    for entry in entries {
        let path = entry?.path();
        if let Some(name) = path.file_name() {
            if let Some(name_str) = name.to_str() {
                if !name_str.contains("part") {
                    devices.push(name_str.to_owned());
                }
            }
        }
    }

    // Print the list of available devices and ask the user to select one
    println!("Available drives:");
    for (i, device) in devices.iter().enumerate() {
        println!("  {}) {}", i + 1, device);
    }

    // Prompt user to select a drive and read their input
    print!("Enter the number of the drive you want to use: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let index = input.trim().parse::<usize>().map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    // Return the selected device
    let selected_device = devices.get(index - 1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid selection"))?;

    Ok(selected_device.clone())
}

// This function partitions the specified drive into two partitions for EFI and ZFS, formats the EFI partition with FAT32, and erases all data on the drive. The function takes the drive's name as input and returns a `String` indicating the completion of the operation.
fn zfs_partition_drive(drive: &str) -> std::io::Result<String> {
    // Define a vector of commands to execute
    let commands = vec![
        format!("blkdiscard -f /dev/disk/by-id/{}", drive), // Erase all data on the drive
        format!("sgdisk -n 1:0:+512M -t 1:EF00 -c 1:EFI /dev/disk/by-id/{}",drive), // Create a 512MB partition for EFI
        format!("sgdisk -n 2:0:0 -t 2:BF01 -c 2:ZFS /dev/disk/by-id/{}", drive), // Create a partition for ZFS
        format!("mkfs.vfat -F32 /dev/disk/by-id/{}-part1", drive), // Format the EFI partition with FAT32
    ];

    // Iterate through the vector of commands and execute them sequentially
    for command in commands {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command");

        // If the command fails, print an error message and exit the program
        if !output.status.success() {
            eprintln!("Command '{}' failed with exit status: {:?}", command, output.status);
            std::process::exit(1);
        }

        // Print the output of the command to the console
        println!("{}", String::from_utf8_lossy(&output.stdout));

        // Wait for 3 seconds before executing the next command
        thread::sleep(Duration::from_secs(3));
    }

    // Return a message indicating that the disk has been formatted
    Ok(format!("Disk Formatted"))
}

// This function creates a ZFS filesystem on the specified drive by executing a sequence of shell commands using `Command` from the standard library. The commands create a zpool, set its properties, create several ZFS datasets, unmount all ZFS datasets, export the zpool, import it into the specified directory, mount the default ZFS dataset, create a boot directory, mount the EFI partition to the boot directory, and create an /etc directory. The function takes the drive's name as input and returns a `String` indicating the completion of the operation.
fn zfs_setup_filesystem(drive: &str) -> std::io::Result<String> {
    // Define a vector of commands to execute
    let commands = vec![
        format!("zpool create -f -o ashift=12 -O canmount=off -O acltype=posixacl -O compression=on -O atime=off -O xattr=sa zroot /dev/disk/by-id/{}-part2", drive), // Create a zpool and set its properties
        "zfs create -o canmount=off -o mountpoint=none zroot/ROOT".to_string(), // Create the ZFS datasets
        "zfs create -o canmount=noauto -o mountpoint=/ zroot/ROOT/default".to_string(),
        "zfs create -o mountpoint=none zroot/data".to_string(),
        "zfs create -o mountpoint=/home zroot/data/home".to_string(),
        "zfs umount -a".to_string(), // Unmount all ZFS datasets
        "zpool export zroot".to_string(), // Export the zpool
        "zpool import -d /dev/disk/by-id -R /mnt zroot".to_string(), // Import the zpool into the specified directory
        "zfs mount zroot/ROOT/default".to_string(), // Mount the default ZFS dataset
        "zpool set bootfs=zroot/ROOT/default zroot".to_string(), // Set the bootfs property of the zpool
        "mkdir /mnt/boot".to_string(), // Create a boot directory
        format!("mount /dev/disk/by-id/{}-part1 /mnt/boot", drive), // Mount the EFI partition to the boot directory
        "mkdir /mnt/etc".to_string(), // Create an /etc directory
    ];

    // Iterate through the vector of commands and execute them sequentially
    for command in commands {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command");

        // If the command fails, print an error message and exit the program
        if !output.status.success() {
            eprintln!("Command '{}' failed with exit status: {:?}", command, output.status);
            std::process::exit(1);
        }

        // Print the output of the command to the console
        println!("{}", String::from_utf8_lossy(&output.stdout));

        // Wait for 3 seconds before executing the next command
        thread::sleep(Duration::from_secs(3));
    }

    // Return a message indicating that the ZFS filesystem has been set up
    Ok(format!("Setup ZFS Filesystem"))
}

// This function sets up a base system on the ZFS filesystem by executing a sequence of shell commands using `Command` from the standard library. The commands generate the fstab file, install packages, and copy the installation script to the ZFS filesystem. The function takes no input and returns a `String` indicating the completion of the operation.
fn zfs_setup_basesystem() -> std::io::Result<String> {
    // Define a vector of commands to execute
    let commands = vec![
        "genfstab -U /mnt >> /mnt/etc/fstab".to_string(), // Generate the fstab file
        "pacstrap /mnt base base-devel linux linux-firmware neovim networkmanager intel-ucode".to_string(), // Install packages
        "cp install /mnt/install".to_string(), // Copy the installation script to the ZFS filesystem
    ];

    // Iterate through the vector of commands and execute them sequentially
    for command in commands {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command");

        // If the command fails, print an error message and exit the program
        if !output.status.success() {
            eprintln!("Command '{}' failed with exit status: {:?}", command, output.status);
            std::process::exit(1);
        }

        // Print the output of the command to the console
        println!("{}", String::from_utf8_lossy(&output.stdout));

        // Wait for 3 seconds before executing the next command
        thread::sleep(Duration::from_secs(3));
    }

    // Return a message indicating that the base system setup is complete
    Ok(format!("Setup basesystem done"))
}

// This function runs a chroot environment by executing a sequence of shell commands using `Command` from the standard library. The commands prompt the user for a username and password, install packages, and configure the ZFS filesystem. The function takes no input and returns nothing.

fn chroot() {
    // Prompt the user for a username and password
    print!("Enter username: ");
    io::stdout().flush();
    let mut username = String::new();
    io::stdin().read_line(&mut username);

    print!("Enter password: ");
    io::stdout().flush();
    let mut password = String::new();
    io::stdin().read_line(&mut password);

    // Call the chroot_install() function to install packages and configure the ZFS filesystem
    chroot_install(username.trim(), password.trim());

    // Exit the program
    std::process::exit(0);
}

// This function installs packages and configures the ZFS filesystem in a chroot environment by executing a sequence of shell commands using `Command` from the standard library. The commands add a repository, install packages, create a user, set up a cache file, configure the bootloader, enable services, and generate an initramfs. The function takes a username and password as input and returns a `String` indicating the completion of the operation.

fn chroot_install(username: &str, password: &str) -> std::io::Result<String> {
    // Define a vector of shell commands to execute
    let commands = vec![
        "echo -e '[archzfs]\nServer = https://archzfs.com/$repo/$arch' >>/etc/pacman.conf".to_string(), // Add a repository
        "pacman-key -r DDF7DB817396A49B2A2723F7403BD972F75D9D76".to_string(), // Add a repository key
        "pacman-key --lsign-key DDF7DB817396A49B2A2723F7403BD972F75D9D76".to_string(), // Sign the repository key
        "pacman -Syu --noconfirm".to_string(), // Update the system
        "pacman -S --noconfirm nfs-utils linux-headers zfs-dkms openssh networkmanager fish git".to_string(), // Install packages
        format!("useradd -m -G wheel -s /usr/bin/fish {}", username), // Create a user
        format!("(echo '{}'; echo '{}') | passwd {}", password, password, username), // Set the user password
        "zpool set cachefile=/etc/zfs/zpool.cache zroot".to_string(), // Set up the cache file
        "bootctl install".to_string(), // Install the bootloader
        "systemctl enable NetworkManager".to_string(), // Enable NetworkManager service
        "echo -e 'title Arch Linux\nlinux vmlinuz-linux\ninitrd intel-ucode.img\ninitrd initramfs-linux.img\noptions zfs=zroot/ROOT/default rw' > /boot/loader/entries/arch.conf".to_string(), // Configure the bootloader
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
        let output = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output()
                .expect("Failed to execute command");

        // Check if the command was successful and print its output
        if !output.status.success() {
            eprintln!("Command '{}' failed with exit status: {:?}", command, output.status);
            std::process::exit(1);
        }
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }

    // Return a `String` indicating the completion of the operation
    Ok(format!("Chroot Install Done"))
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
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command");

        // Check if the command was successful and print its output
        if !output.status.success() {
            eprintln!("Command '{}' failed with exit status: {:?}", command, output.status);
            std::process::exit(1);
        }
        println!("{}", String::from_utf8_lossy(&output.stdout));
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
        "yay -Syu --noconfirm --answerdiff=None kitty ripgrep unzip bat pavucontrol pipewire-pulse dunst bluedevil bluez-utils brightnessctl grimblast-git neovim network-manager-applet rofi-lbonn-wayland-git starship thunar thunar-archive-plugin thunar-volman  webcord-bin wl-clipboard librewolf-bin neofetch swaybg waybar-hyprland-git btop tldr swaylock-effects obsidian fish hyprland-bin npm xdg-desktop-portal-hyprland-git exa noto-fonts-emoji qt5-wayland qt6-wayland blueman swappy playerctl wlogout sddm-git nano ttf-jetbrains-mono-nerd lazygit swayidle".to_string(),
    ];

    for command in commands {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command");

        if !output.status.success() {
            // If a command fails, print an error message and exit
            eprintln!("Command '{}' failed with exit status: {:?}", command, output.status);
            std::process::exit(1);
        }

        // Print the output of the command
        println!("{}", String::from_utf8_lossy(&output.stdout));
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
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command"); // Execute the command and print an error message if it fails

        // If the command fails, print an error message and exit with status code 1
        if !output.status.success() {
            eprintln!("Command '{}' failed with exit status: {:?}", command, output.status);
            std::process::exit(1);
        }

        println!("{}", String::from_utf8_lossy(&output.stdout)); // Print the output of the command
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
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command"); // Execute the command and print an error message if it fails

        // If the command fails, print an error message and exit with status code 1
        if !output.status.success() {
            eprintln!("Command '{}' failed with exit status: {:?}", command, output.status);
            std::process::exit(1);
        }

        println!("{}", String::from_utf8_lossy(&output.stdout)); // Print the output of the command
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
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command"); // Execute the command and print an error message if it fails

        // If the command fails, print an error message and exit with status code 1
        if !output.status.success() {
            eprintln!("Command '{}' failed with exit status: {:?}", command, output.status);
            std::process::exit(1);
        }

        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
    Ok(format!("Stetsed Extra's Done")) // Return a success message
}

