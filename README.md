# Stetsed's ZFS Arch Linux Installer

Welcome to a little side project I am doing which is implementing my previosly Bash script found [here](https://github.com/Stetsed/.dotfiles/blob/main/.bin/fullinstall.sh), into Rust. Sadly what I found is that most of the stuff that I am doing requires me to just execute it via the terminal. But I have tried to go with Rust Native functions when I can. This is my first time using Rust so it's still a learning journey.

The first part of this script that installs the ZFS volume and the chroot/base system should be compatible with any system. After this the "User" stage will install the packages that I need for my dotfiles. So you may or may not want to use this stage.

## Steps

### Setup ZFS

In this step we first download the script provided by [eoli3n](https://raw.githubusercontent.com/eoli3n/archiso-zfs/master/init) so that we can do ZFS functions in the ArchIso. After this we let the user select a drive by the selection found in /dev/disk/by-id. After this we wipe the drive, after this we create a 512MB EFI partition and then create a main partition with the rest of the drive. 

After all of this we create the ZFS pool and the necesarry volumes such as ROOT and home. We then mount these to the /mnt location and install the base packagers and then copy the install script to the root of that and then this stage is done.

### Setup Chroot

First we ask for the username and password the user wants that will be used to create the user.After this we setup the archzfs repository to allow for the installation of packages. After this we install the zfs-dkms and linux-headers and a few other required packages. Then we create our user and set the password.

After this we set the zpool cachefile, install the bootloader, enable networkmanager, add our systemd-boot entry. We set the default for systemd-boot, we make it so wheel users have sudo acces, enable some ZFS services, add zfs to the mkinitcpio.conf and rebuild them. And then we are done with the Chroot Stage.

## Setup User

In this function we start by making our home directory because of some apparent ZFS problems our home directory gets deleted. We go ahead and make our home directory. Now we go ahead and install the yay AUR helper to be able to install AUR packages. And then we install the packages that belong to my dotfiles, this may be changed in the future (Check To Do). 

After this you can either choose to install my Dotfiles, or enter your own dotfiles repository which should work aslong as it's in a bare repository format. After this you are asked if you are Stetsed for some more personal configuration otherwise the program exits and you are done.



## To-Do

- [x] ZFS Stage
- [x] Chroot Stage
- [x] User Stage
- [] Grab package list from repository
- [] Try to use more rust native code instead of terminal commands.
