use std::fs;
use std::io::{self, Write};
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn zfs() {
    // Call the necessary sub-functions in the correct order.
    zfs_get_zfs().expect("Failed to install ZFS");
    let selected_drive = zfs_select_drive().unwrap_or_else(|err| {
        eprintln!("Failed to select drive: {}", err);
        String::new()
    });
    zfs_partition_drive(&selected_drive).expect("Failed to partition drive");
    zfs_setup_filesystem(&selected_drive).expect("Failed to setup filesystem");
    zfs_setup_basesystem().expect("Failed to setup basesystem");

    // Exit the program with a successful status code.
    std::process::exit(0);
}

// Function to download and install ZFS.
pub fn zfs_get_zfs() -> std::io::Result<String> {
    // Execute the curl command to download the ZFS installation script.
    let output = Command::new("curl")
        .args(&[
            "-s",
            "https://raw.githubusercontent.com/eoli3n/archiso-zfs/master/init",
        ])
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
pub fn zfs_select_drive() -> Result<String, std::io::Error> {
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
    let index = input
        .trim()
        .parse::<usize>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    // Return the selected device
    let selected_device = devices
        .get(index - 1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid selection"))?;

    Ok(selected_device.clone())
}

// This function partitions the specified drive into two partitions for EFI and ZFS, formats the EFI partition with FAT32, and erases all data on the drive. The function takes the drive's name as input and returns a `String` indicating the completion of the operation.
pub fn zfs_partition_drive(drive: &str) -> std::io::Result<String> {
    // Define a vector of commands to execute
    let commands = vec![
        format!("blkdiscard -f /dev/disk/by-id/{}", drive), // Erase all data on the drive
        format!(
            "sgdisk -n 1:0:+512M -t 1:EF00 -c 1:EFI /dev/disk/by-id/{}",
            drive
        ), // Create a 512MB partition for EFI
        format!(
            "sgdisk -n 2:0:0 -t 2:BF01 -c 2:ZFS /dev/disk/by-id/{}",
            drive
        ), // Create a partition for ZFS
        format!("mkfs.vfat -F32 /dev/disk/by-id/{}-part1", drive), // Format the EFI partition with FAT32
    ];

    // Iterate through the vector of commands and execute them sequentially
    for command in commands {
        execute_command(&command)?;
    }

    // Return a message indicating that the disk has been formatted
    Ok(format!("Disk Formatted"))
}

// This function creates a ZFS filesystem on the specified drive by executing a sequence of shell commands using `Command` from the standard library. The commands create a zpool, set its properties, create several ZFS datasets, unmount all ZFS datasets, export the zpool, import it into the specified directory, mount the default ZFS dataset, create a boot directory, mount the EFI partition to the boot directory, and create an /etc directory. The function takes the drive's name as input and returns a `String` indicating the completion of the operation.
pub fn zfs_setup_filesystem(drive: &str) -> std::io::Result<String> {
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
        execute_command(&command)?;
    }

    // Return a message indicating that the ZFS filesystem has been set up
    Ok(format!("Setup ZFS Filesystem"))
}

// This function sets up a base system on the ZFS filesystem by executing a sequence of shell commands using `Command` from the standard library. The commands generate the fstab file, install packages, and copy the installation script to the ZFS filesystem. The function takes no input and returns a `String` indicating the completion of the operation.
fn zfs_setup_basesystem() -> std::io::Result<String> {
    // Define a vector of commands to execute
    let commands = vec![
        "genfstab -U /mnt >> /mnt/etc/fstab".to_string(), // Generate the fstab file
        "pacstrap /mnt base base-devel linux linux-firmware neovim networkmanager".to_string(), // Install packages
        "cp install /mnt/install".to_string(), // Copy the installation script to the ZFS filesystem
    ];

    // Iterate through the vector of commands and execute them sequentially
    for command in commands {
        execute_command(&command)?;
    }

    // Return a message indicating that the base system setup is complete
    Ok(format!("Setup basesystem done"))
}

pub fn execute_command(command: &str) -> std::io::Result<()> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        eprintln!(
            "Command '{}' failed with exit status: {:?}",
            command, output.status
        );
        std::process::exit(1);
    }

    println!("{}", String::from_utf8_lossy(&output.stdout));

    thread::sleep(Duration::from_secs(3));

    Ok(())
}
