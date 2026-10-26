#!/usr/bin/env python3
"""
PS5 Homebrew Package Builder
Builds a proper .pkg file for PS5 that appears in the Media tab.
"""

import os
import sys
import shutil
import subprocess
import argparse
from pathlib import Path

def generate_param_sfo(output_path: Path, title_id: str, title: str, version: str, app_ver: str, content_id: str, category: str = "media", system_ver: str = "5.00"):
    """Generate param.sfo in text format (key=value) for PS5 packaging tools."""
    lines = [
        f"TITLE_ID={title_id}",
        f"TITLE={title}",
        f"TITLE_NAME={title}",
        f"CATEGORY={category}",
        f"VERSION={version}",
        f"APP_VER={app_ver}",
        f"CONTENT_ID={content_id}",
        f"SYSTEM_VER={system_ver}",
        "ATTRIBUTE=0",
        "PARENTAL_LEVEL=1",
    ]
    content = "\n".join(lines) + "\n"
    output_path.write_text(content)
    print(f"Generated param.sfo at {output_path}")

def create_package_structure(
    binary_path: Path,
    output_dir: Path,
    title_id: str,
    title: str,
    version: str,
    app_ver: str,
    content_id: str,
    icon_path: Path = None,
    category: str = "media"
):
    """Create the PS5 package directory structure."""
    pkg_dir = output_dir / "package"
    sce_sys_dir = pkg_dir / "sce_sys"

    # Clean and create directories
    if pkg_dir.exists():
        shutil.rmtree(pkg_dir)
    sce_sys_dir.mkdir(parents=True)

    # Copy binary as eboot.bin
    eboot_path = pkg_dir / "eboot.bin"
    shutil.copy2(binary_path, eboot_path)
    print(f"Copied binary to {eboot_path}")

    # Generate param.sfo
    param_sfo_path = sce_sys_dir / "param.sfo"
    generate_param_sfo(param_sfo_path, title_id, title, version, app_ver, content_id, category)

    # Copy or create icon
    icon_dest = sce_sys_dir / "icon0.png"
    if icon_path and icon_path.exists():
        shutil.copy2(icon_path, icon_dest)
        print(f"Copied icon to {icon_dest}")
    else:
        # Create a simple placeholder icon (512x512 PNG)
        create_placeholder_icon(icon_dest)
        print(f"Created placeholder icon at {icon_dest}")

    return pkg_dir

def create_placeholder_icon(output_path: Path):
    """Create a simple placeholder icon using PIL if available, or just create a minimal PNG."""
    try:
        from PIL import Image, ImageDraw
        # Create a 512x512 Discord-themed icon
        img = Image.new('RGBA', (512, 512), (0x58, 0x65, 0xF2, 255))  # Discord blurple
        draw = ImageDraw.Draw(img)

        # Draw a simple "D" logo (Discord style)
        # Outer white circle
        draw.ellipse([64, 64, 448, 448], fill=(0xFF, 0xFF, 0xFF, 255))
        # Inner blurple circle
        draw.ellipse([128, 128, 384, 384], fill=(0x58, 0x65, 0xF2, 255))
        # Vertical white bar (left side of D)
        draw.rectangle([180, 128, 220, 384], fill=(0xFF, 0xFF, 0xFF, 255))
        # Top and bottom curves of D
        draw.ellipse([180, 128, 332, 280], fill=(0x58, 0x65, 0xF2, 255))
        draw.ellipse([180, 232, 332, 384], fill=(0x58, 0x65, 0xF2, 255))
        # Right side of D
        draw.rectangle([280, 180, 332, 332], fill=(0xFF, 0xFF, 0xFF, 255))

        img.save(output_path, 'PNG')
        print("Created Discord-themed placeholder icon")
    except ImportError:
        # Create minimal valid PNG if PIL not available
        # This creates a minimal 512x512 blue PNG
        import zlib
        width, height = 512, 512
        # PNG header
        png = b'\x89PNG\r\n\x1a\n'
        # IHDR chunk
        ihdr = b'IHDR' + width.to_bytes(4, 'big') + height.to_bytes(4, 'big') + b'\x08\x02\x00\x00\x00'
        ihdr_crc = zlib.crc32(ihdr) & 0xffffffff
        png += len(ihdr[4:]).to_bytes(4, 'big') + ihdr + ihdr_crc.to_bytes(4, 'big')
        # IDAT chunk (compressed data for solid color)
        # Simple uncompressed scanline: filter type 0 + 512 pixels of RGBA (0x58, 0x65, 0xF2, 0xFF)
        scanline = b'\x00' + bytes([0x58, 0x65, 0xF2, 0xFF]) * width
        raw_data = scanline * height
        compressed = zlib.compress(raw_data)
        idat = b'IDAT' + compressed
        idat_crc = zlib.crc32(idat) & 0xffffffff
        png += len(compressed).to_bytes(4, 'big') + idat + idat_crc.to_bytes(4, 'big')
        # IEND chunk
        iend = b'IEND'
        iend_crc = zlib.crc32(iend) & 0xffffffff
        png += b'\x00\x00\x00\x00' + iend + iend_crc.to_bytes(4, 'big')
        output_path.write_bytes(png)
        print("Warning: PIL not available, created minimal placeholder icon")

def build_pkg(pkg_dir: Path, output_pkg: Path, ps5sdk_dir: Path = None):
    """Build the .pkg file from package directory."""
    # Try to find packaging tool
    pkg_tool = None
    if ps5sdk_dir:
        possible_tools = [
            ps5sdk_dir / "bin" / "pkg",
            ps5sdk_dir / "bin" / "orbis-pub-gen",
            ps5sdk_dir / "bin" / "pkg-python",
            ps5sdk_dir / "bin" / "ps5-pkg-tool",
        ]
        for tool in possible_tools:
            if tool.exists():
                pkg_tool = tool
                break

    # Also check PATH
    if not pkg_tool:
        for tool_name in ["pkg", "orbis-pub-gen", "pkg-python", "ps5-pkg-tool", "ps5-pkg"]:
            result = shutil.which(tool_name)
            if result:
                pkg_tool = Path(result)
                break

    if pkg_tool:
        print(f"Using packaging tool: {pkg_tool}")
        try:
            tool_name = str(pkg_tool).lower()
            if "pkg-python" in tool_name:
                # pkg-python create -i <input_dir> -o <output.pkg>
                subprocess.run([str(pkg_tool), "create", "-i", str(pkg_dir), "-o", str(output_pkg)], check=True)
            elif "orbis-pub-gen" in tool_name:
                print("orbis-pub-gen is a GUI tool, cannot run headless")
                return False
            elif "ps5-pkg" in tool_name:
                # ps5-pkg-tool create <input_dir> <output.pkg>
                subprocess.run([str(pkg_tool), "create", str(pkg_dir), str(output_pkg)], check=True)
            else:
                # Generic attempt
                subprocess.run([str(pkg_tool), "create", str(pkg_dir), str(output_pkg)], check=True)
            print(f"Created package: {output_pkg}")
            return True
        except subprocess.CalledProcessError as e:
            print(f"Packaging failed: {e}")
            return False
    else:
        print("No packaging tool found in PS5SDK or PATH.")
        print("Package directory structure created at:", pkg_dir)
        print("\nTo create the .pkg manually, you can use:")
        print("  1. orbis-pub-gen (GUI) - Open the package directory and build")
        print("  2. pkg-python (CLI) - pkg-python create -i package -o discord-ps5.pkg")
        print("  3. ps5-pkg-tool (CLI) - ps5-pkg-tool create package discord-ps5.pkg")
        print("\nPackage structure:")
        for root, dirs, files in os.walk(pkg_dir):
            for f in files:
                fp = Path(root) / f
                rel = fp.relative_to(pkg_dir)
                print(f"  {rel}")
        return False

def main():
    parser = argparse.ArgumentParser(description="Build PS5 homebrew package")
    parser.add_argument("binary", help="Path to the ELF binary")
    parser.add_argument("-o", "--output", default="discord-ps5.pkg", help="Output .pkg file")
    parser.add_argument("--title-id", default="DISCORDPS5", help="Title ID (max 16 chars)")
    parser.add_argument("--title", default="Discord PS5", help="Title (max 128 chars)")
    parser.add_argument("--version", default="0.1.0", help="Version string")
    parser.add_argument("--app-ver", default="01.00", help="App version (format: XX.XX)")
    parser.add_argument("--content-id", default="DISCORDPS5_001", help="Content ID")
    parser.add_argument("--category", default="media", help="Category (media, game, etc.)")
    parser.add_argument("--system-ver", default="5.00", help="Minimum system version")
    parser.add_argument("--icon", help="Path to icon0.png (512x512)")
    parser.add_argument("--ps5sdk", help="Path to PS5SDK (for packaging tools)")
    parser.add_argument("--work-dir", default="build/pkg", help="Working directory for package structure")

    args = parser.parse_args()

    binary_path = Path(args.binary).resolve()
    if not binary_path.exists():
        print(f"Error: Binary not found: {binary_path}")
        return 1

    output_pkg = Path(args.output).resolve()
    work_dir = Path(args.work_dir).resolve()
    ps5sdk_dir = Path(args.ps5sdk).resolve() if args.ps5sdk else None
    icon_path = Path(args.icon).resolve() if args.icon else None

    print(f"Building PS5 package for {binary_path}")
    print(f"Output: {output_pkg}")
    print(f"Title ID: {args.title_id}")
    print(f"Title: {args.title}")
    print(f"Version: {args.version}")
    print(f"Category: {args.category} (appears in Media tab when set to 'media')")

    # Create package structure
    pkg_dir = create_package_structure(
        binary_path=binary_path,
        output_dir=work_dir,
        title_id=args.title_id,
        title=args.title,
        version=args.version,
        app_ver=args.app_ver,
        content_id=args.content_id,
        icon_path=icon_path,
        category=args.category
    )

    # Try to build .pkg
    success = build_pkg(pkg_dir, output_pkg, ps5sdk_dir)

    if success:
        print(f"\nSuccess! Package created: {output_pkg}")
        print("Install on PS5 via package installer to appear in Media tab")
    else:
        print(f"\nPackage structure created at: {pkg_dir}")
        print("Use orbis-pub-gen or pkg-python to create final .pkg")

    return 0 if success else 1

if __name__ == "__main__":
    sys.exit(main())