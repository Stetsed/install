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
                "--transfer" => transfer(),
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
    println!("4. Transfer");

    let mut choice = String::new();

    io::stdin().read_line(&mut choice).expect("Failed to read line");

    match choice.trim() {
        "1" => zfs(),
        "2" => chroot(),
        "3" => user(),
        "4" => transfer(),
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

    println!("ZFS and Chroot is now finished, press enter to reboot");

    // Wait for user input
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();

    // Reboot the system
    let output = Command::new("sh")
        .arg("-c")
        .arg("reboot")
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        eprintln!("Reboot failed with exit status: {:?}", output.status);
        std::process::exit(1);
    }
}

fn zfs_get_zfs() -> std::io::Result<String> {
    let output = Command::new("curl")
        .arg("-s")
        .arg("https://raw.githubusercontent.com/eoli3n/archiso-zfs/master/init")
        .stdout(std::process::Stdio::piped())
        .spawn()?
        .stdout
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Could not capture stdout"))?;

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
        format!("zpool labelclear -f /dev/disk/by-id/{}", drive),
        format!("blkdiscard /dev/disk/by-id/{}", drive),
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
        format!("zpool create -f -o ashift=12 -O canmount=off -O acltype=posixacl -O compression=on -O atime=off -O xattr=sa zroot /dev/disk/by-id/{}", drive),
        "zfs create -o canmount=off -o mountpoint=none zroot/ROOT".to_string(),
        "zfs create -o canmount=noauto -o mountpoint=/ zroot/ROOT/default".to_string(),
        "zfs create -o mountpoint=none zroot/data".to_string(),
        "zfs create -o mountpoint=/home zroot/data/home".to_string(),
        "zfs umount -a".to_string(),
        "zpool export zroot".to_string(),
        "zpool import -d /dev/disk/by-id -R /mnt zroot".to_string(),
        "zfs mount zroot/ROOT/default".to_string(),
        "zfs mount zroot/data/home".to_string(),
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
        "arch-chroot /mnt /install --chroot".to_string(),
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
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;

    print!("Enter password: ");
    io::stdout().flush()?;
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;

    chroot_install(username.trim(), password.trim())?;

    chroot_install();
}

fn chroot_install(username: &str, password: &str) -> std::io::Result<String> {
    let commands = vec![
        "echo -e '[archzfs]\nServer = https://archzfs.com/$repo/$arch' >>/etc/pacman.conf".to_string(),
        "pacman-key -r DDF7DB817396A49B2A2723F7403BD972F75D9D76".to_string(),
        "pacman-key --lsign-key DDF7DB817396A49B2A2723F7403BD972F75D9D76".to_string(),
        "pacman -Syu --noconfirm".to_string(),
        "pacman -S linux-headers zfs-dkms openssh networkmanager fish".to_string(),
        format!("useradd -m -G wheel -s /usr/bin/fish {}", username),
        format!("(echo '{}'; echo '{}') | passwd {}", password, password, username),
        "zpool set cachefile=/etc/zfs/zpool.cache".to_string(),
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

        thread::sleep(Duration::from_secs(3));
    }


    Ok(format!("Chroot Install Done"))
}

fn user() {

}

fn transfer() {
    println!("You chose Transfer");
    // Run Transfer function here
}


