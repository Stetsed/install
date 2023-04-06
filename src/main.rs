use std::io::{self, Write};
use std::env;
use std::process::Command;
use std::fs;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--zfs" => zfs(),
                "--chroot" => chroot(),
                "--user" => user(),
                _ => print!("Invalid Flag Passed"),
            }
        }
    } else {
        no_flag_passed();
    }
}

fn no_flag_passed(){
    println!("Choose an option:");
    println!("1. ZFS");
    println!("2. Chroot");
    println!("3. User");

    let mut choice = String::new();

    io::stdin().read_line(&mut choice).expect("Failed to read line");

    match choice.trim() {
        "1" => zfs(),
        "2" => chroot(),
        "3" => user(),
        _ => println!("Invalid choice"),
    }
}

fn zfs() {
    zfs_get_zfs();

    let selected_drive = zfs_select_drive().unwrap_or_else(|err| {
        eprintln!("Failed to select drive: {}", err);
        String::new()
    });

    zfs_partition_drive(&selected_drive);

    zfs_setup_filesystem(&selected_drive);

    zfs_setup_basesystem();

    std::process::exit(0);


}

fn zfs_get_zfs() -> std::io::Result<String> {
    let output = Command::new("curl")
        .args(&["-s", "https://raw.githubusercontent.com/eoli3n/archiso-zfs/master/init"])
        .output()
        .expect("failed to execute curl command");

    let output_str = String::from_utf8_lossy(&output.stdout);

    Command::new("bash")
        .arg("-c")
        .arg(output_str.trim())
        .status()
        .expect("failed to execute bash command");

    Ok(format!("Installed ZFS"))
}


fn zfs_select_drive() -> Result<String, std::io::Error> {
    let devices_dir = "/dev/disk/by-id";

    // Get a list of devices in the directory
    let entries = fs::read_dir(devices_dir)?;

    let mut devices = Vec::new();
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

    // Print the list of devices and ask the user to select one
    println!("Available drives:");
    for (i, device) in devices.iter().enumerate() {
        println!("  {}) {}", i + 1, device);
    }

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

fn zfs_partition_drive(drive: &str) -> std::io::Result<String> {
    let commands = vec![
        format!("blkdiscard -f /dev/disk/by-id/{}", drive),
        format!("sgdisk -n 1:0:+512M -t 1:EF00 -c 1:EFI /dev/disk/by-id/{}",drive),
        format!("sgdisk -n 2:0:0 -t 2:BF01 -c 2:ZFS /dev/disk/by-id/{}", drive),
        format!("mkfs.vfat -F32 /dev/disk/by-id/{}-part1", drive),
    ];

    for command in commands {
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
    }
    Ok(format!("Disk Formatted"))
}

fn zfs_setup_filesystem(drive: &str) -> std::io::Result<String> {
    let commands = vec![
        format!("zpool create -f -o ashift=12 -O canmount=off -O acltype=posixacl -O compression=on -O atime=off -O xattr=sa zroot /dev/disk/by-id/{}-part2", drive),
        "zfs create -o canmount=off -o mountpoint=none zroot/ROOT".to_string(),
        "zfs create -o canmount=noauto -o mountpoint=/ zroot/ROOT/default".to_string(),
        "zfs create -o mountpoint=none zroot/data".to_string(),
        "zfs create -o mountpoint=/home zroot/data/home".to_string(),
        "zfs umount -a".to_string(),
        "zpool export zroot".to_string(),
        "zpool import -d /dev/disk/by-id -R /mnt zroot".to_string(),
        "zfs mount zroot/ROOT/default".to_string(),
        "zpool set bootfs=zroot/ROOT/default zroot".to_string(),
        "mkdir /mnt/boot".to_string(),
        format!("mount /dev/disk/by-id/{}-part1 /mnt/boot", drive),
        "mkdir /mnt/etc".to_string(),
    ];

    for command in commands {
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
    }


    Ok(format!("Setup ZFS Filesystem"))
}

fn zfs_setup_basesystem() -> std::io::Result<String> {
    let commands = vec![
        "genfstab -U /mnt >> /mnt/etc/fstab".to_string(),
        "pacstrap /mnt base base-devel linux linux-firmware neovim networkmanager intel-ucode".to_string(),
        "cp install /mnt/install".to_string(),
    ];

    for command in commands {
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
    }


    Ok(format!("Setup basesystem done"))
}

fn chroot() {
    print!("Enter username: ");
    io::stdout().flush();
    let mut username = String::new();
    io::stdin().read_line(&mut username);

    print!("Enter password: ");
    io::stdout().flush();
    let mut password = String::new();
    io::stdin().read_line(&mut password);

    chroot_install(username.trim(), password.trim());

    std::process::exit(0);
}

fn chroot_install(username: &str, password: &str) -> std::io::Result<String> {
    let commands = vec![
        "echo -e '[archzfs]\nServer = https://archzfs.com/$repo/$arch' >>/etc/pacman.conf".to_string(),
        "pacman-key -r DDF7DB817396A49B2A2723F7403BD972F75D9D76".to_string(),
        "pacman-key --lsign-key DDF7DB817396A49B2A2723F7403BD972F75D9D76".to_string(),
        "pacman -Syu --noconfirm".to_string(),
        "pacman -S --noconfirm nfs-utils linux-headers zfs-dkms openssh networkmanager fish git".to_string(),
        format!("useradd -m -G wheel -s /usr/bin/fish {}", username),
        format!("(echo '{}'; echo '{}') | passwd {}", password, password, username),
        "zpool set cachefile=/etc/zfs/zpool.cache zroot".to_string(),
        "bootctl install".to_string(),
        "systemctl enable NetworkManager".to_string(),
        "echo -e 'title Arch Linux\nlinux vmlinuz-linux\ninitrd intel-ucode.img\ninitrd initramfs-linux.img\noptions zfs=zroot/ROOT/default rw' > /boot/loader/entries/arch.conf".to_string(),
        "echo 'default arch' >> /boot/loader/loader.conf".to_string(),
        "echo '%wheel ALL=(ALL:ALL) ALL' >> /etc/sudoers".to_string(),
        "systemctl enable zfs-scrub-weekly@zroot.timer".to_string(),
        "systemctl enable zfs.target".to_string(),
        "systemctl enable zfs-import-cache".to_string(),
        "systemctl enable zfs-mount".to_string(),
        "zgenhostid $(hostid)".to_string(),
        "sed -i 's/keyboard keymap/keyboard zfs keymap/g' /etc/mkinitcpio.conf".to_string(),
        "mkinitcpio -P".to_string(),
    ];

    for command in commands {
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
    }


    Ok(format!("Chroot Install Done"))
}


fn user(){
    user_create_home();

    user_yay_packages();

    user_install_dotfiles();

    user_extras();

    // Ask the user if they want to use Stetsed's Dotfiles
    let mut input = String::new();
    print!("Do you want to use Stetsed's Home Configuration(Say no if your not Stetsed)? (y/n): ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();

    if input.trim().eq_ignore_ascii_case("y") {
        user_extras_stetsed();
    }
    
    print!("Thank you for using Stetsed's Installer! Hope it helped you! :");

    std::process::exit(0);
}

fn user_create_home() -> std::io::Result<String>  {
   let output = Command::new("whoami")
        .output()
        .expect("failed to execute process");

    let whoami_output = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let commands = vec![
        format!("sudo mkdir /home/{}", whoami_output),
        format!("sudo chown {}:{} -R /home/{}", whoami_output, whoami_output, whoami_output),
        format!("sudo chmod 700 /home/{}", whoami_output),
    ];

    for command in commands {
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
    }
    Ok(format!("Home Created"))
}


fn user_yay_packages() -> std::io::Result<String>  {
   let output = Command::new("whoami")
        .output()
        .expect("failed to execute process");

    let whoami_output = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let commands = vec![
        format!("cd /home/{} && git clone https://aur.archlinux.org/yay-bin.git", whoami_output),
        format!("cd /home/{} && cd yay-bin && makepkg -s && sudo pacman -U --noconfirm yay-bin* && cd .. && rm -rf yay-bin", whoami_output),
        "yay -Syu --noconfirm --answerdiff=None kitty ripgrep unzip bat pavucontrol pipewire-pulse dunst bluedevil bluez-utils brightnessctl grimblast-git neovim network-manager-applet rofi-lbonn-wayland-git starship thunar thunar-archive-plugin thunar-volman  webcord-bin wl-clipboard librewolf-bin neofetch swaybg waybar-hyprland-git btop tldr swaylock-effects obsidian fish hyprland-bin npm xdg-desktop-portal-hyprland-git exa noto-fonts-emoji qt5-wayland qt6-wayland blueman swappy playerctl wlogout sddm-git nano ttf-jetbrains-mono-nerd lazygit swayidle".to_string(),
    ];

    for command in commands {
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
    }
    Ok(format!("Yay and Packages Installed"))
}

fn user_install_dotfiles() -> std::io::Result<String> {
    let mut dotfiles_url = String::new();
    let mut ssh_url = String::new();
    // Ask the user if they want to use Stetsed's Dotfiles
    let mut input = String::new();
    print!("Do you want to use Stetsed's Dotfiles? (y/n): ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();

    if input.trim().eq_ignore_ascii_case("y") {
        dotfiles_url = "https://github.com/Stetsed/.dotfiles.git".to_owned();
        ssh_url = "git@github.com:Stetsed/.dotfiles.git".to_owned();
    } else {
        // Ask the user for their github dotfiles repository
        print!("Enter your github dotfiles repository (username/repository_name): ");
        let mut dotfiles_repo = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut dotfiles_repo).unwrap();
        let dotfiles_repo_trim = dotfiles_repo.trim().to_owned();
        dotfiles_url = format!("https://github.com/{}.git", dotfiles_repo_trim);
        ssh_url = format!("git@github.com:{}.git", dotfiles_repo_trim);
    }

    let commands = vec![
        format!("git clone --bare {} $HOME/.dotfiles", dotfiles_url),
        "/usr/bin/git --git-dir=$HOME/.dotfiles/ --work-tree=$HOME checkout -f".to_string(),
        "/usr/bin/git --git-dir=$HOME/.dotfiles/ --work-tree=$HOME config status.showUntrackedFiles no".to_string(),
        format!("/usr/bin/git --git-dir=$HOME/.dotfiles/ --work-tree=$HOME config remote set-url origin {}", ssh_url),
    ];

    for command in commands {
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
    }

    Ok(format!("Dotfiles Installed"))
}

fn user_extras() -> std::io::Result<String> {
    let commands = vec![
        "sudo systemctl enable --now bluetooth && sudo systemctl enable sddm",
        "systemctl --user enable --now pipewire",
        "systemctl enable --now pipewire-pulse",
        "sudo timedatectl set-ntp true && sudo timedatectl set-timezone Europe/Amsterdam",
    ];

    for command in commands {
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
    }


    Ok(format!("Extra's Done"))
}

fn user_extras_stetsed() -> std::io::Result<String> {
    let commands = vec![
        "echo '10.4.78.251:/mnt/Vault/Storage /mnt/data nfs defaults,_netdev,x-systemd.automount,x-systemd.mount-timeout=10,noauto 0 0' | sudo tee -a /etc/fstab",
        "sudo mkdir /mnt/data",
        "sudo mount -t nfs 10.4.78.251:/mnt/Vault/Storage /mnt/data",
        "ln -s /mnt/data/Stetsed/Storage ~/Storage",
        "ln -s /mnt/data/Stetsed/Documents ~/Documents",
        "echo -e '[Autologin]\nUser=stetsed\nSession=hyprland' | sudo tee -a /etc/sddm.conf",
        "sudo groupadd autologin && sudo usermod -aG autologin stetsed",
    ];

    for command in commands {
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
    }


    Ok(format!("Stetsed Extra's Done")) 
}
