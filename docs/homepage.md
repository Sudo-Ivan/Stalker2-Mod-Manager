# S.T.A.L.K.E.R. 2 Mod Manager

A simple mod manager for S.T.A.L.K.E.R. 2: Heart of Chornobyl.

## Quick Start Guide

1. Set your game path in Settings
2. Install mods using the "Install Mod" button or drag-and-drop PAK files
3. Enable/disable mods using the switches

## Installing Mods

### From Nexus Mods
![nexus_install](resources/docs/nexus_install.gif)

1. Click "Install Mod"
2. Enter the Nexus Mod ID
3. Click Install

### Local Mods
![local_install](resources/docs/local_install.gif)

You can install local mods in two ways:

#### Using the Install Dialog
1. Click "Install Mod"
2. Choose "Select PAK File"
3. Browse to your mod file
4. Click Open

#### Using Drag and Drop
Simply drag a .pak file from your file explorer into the mod manager window.

## Managing Mods

- Use the switches to enable/disable mods
- Enabled mods are placed in the game's mods folder
- Disabled mods are stored in the ModManager/unloaded_mods folder
- Mod list is automatically saved when closing the application

## Settings

- Game Path: Set the path to your S.T.A.L.K.E.R. 2 installation
- Nexus API Key: Required for installing mods from Nexus Mods
- Import/Export: Backup and restore your mod configuration

## NXM Link Support

The mod manager supports direct installation through NXM links from Nexus Mods. Make sure to:
1. Register the application as the NXM link handler
2. Have a valid Nexus API key configured 