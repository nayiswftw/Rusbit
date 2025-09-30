
<div align="center">
<h1>Rusbit 🦀</h1>
    
**A blazingly fast BitTorrent client written in Rust**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Cargo](https://img.shields.io/badge/cargo-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://doc.rust-lang.org/cargo/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/nayiswftw/Rusbit)

*Download torrents and magnet links from the command line with ease*

<p>
  <a href="#quick-start">🚀 Quick Start</a> •
  <a href="#usage">📖 Documentation</a> •
  <a href="#contributing">🤝 Contributing</a>
</p>

</div>

## ✨ Features 

<div align="center">
  <table>
    <tr>
      <td align="center" width="50%">
        <h3>🔍 <strong>Decode & Parse</strong></h3>
        <p>Parse and decode bencoded values and torrent metadata</p>
        <ul style="list-style-type: none; padding-left: 0;">
          <li>✅ Decode bencoded values</li>
          <li>✅ Torrent info extraction</li>
        </ul>
      </td>
      <td align="center" width="50%">
        <h3>🌐 <strong>Network Operations</strong></h3>
        <p>Peer discovery and protocol handshakes</p>
        <ul style="list-style-type: none; padding-left: 0;">
          <li>✅ Peer discovery</li>
          <li>✅ BitTorrent handshakes</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td align="center" width="50%">
        <h3>📥 <strong>Downloading</strong></h3>
        <p>Flexible download options for pieces and complete files</p>
        <ul style="list-style-type: none; padding-left: 0;">
          <li>✅ Individual piece downloads</li>
          <li>✅ Full torrent downloads</li>
          <li>✅ Concurrent peer downloads</li>
        </ul>
      </td>
      <td align="center" width="50%">
        <h3>🧲 <strong>Magnet Links</strong></h3>
        <p>Complete magnet link support</p>
        <ul style="list-style-type: none; padding-left: 0;">
          <li>✅ Magnet link parsing</li>
          <li>✅ Magnet-based downloads</li>
          <li>✅ Magnet handshakes</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td align="center" colspan="2">
        <h3>⚡ <strong>Performance</strong></h3>
        <p>High-performance asynchronous operations</p>
        <ul style="list-style-type: none; padding-left: 0; display: inline-block;">
          <li>✅ Async downloads with Tokio</li>
          <li>✅ Multi-peer concurrent transfers</li>
          <li>✅ Efficient piece management</li>
        </ul>
      </td>
    </tr>
  </table>
</div>

## 📋 Table of Contents

- [Installation](#installation-)
- [Quick Start](#quick-start)
- [Usage](#usage-)
- [Examples](#examples-)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing-)
- [License](#license-)

## 🚀 Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/nayiswftw/Rusbit.git
cd rusbit-cli

# Build in release mode for optimal performance
cargo build --release
```

The binary will be available at `target/release/rusbit-cli`.

### Verify Installation

```bash
./target/release/rusbit-cli --version
```

## ⚡ Quick Start

Download a torrent file:

```bash
rusbit-cli download -o myfile.txt sample.torrent
```

Download via magnet link:

```bash
rusbit-cli magnet-download -o myfile.txt "magnet:?xt=urn:btih:c5fb9894bdaba464811b088d806bdd611ba490af&dn=magnet3.gif&tr=http%3A%2F%2Fbittorrent-test-tracker.codecrafters.io%2Fannounce"
```

Get torrent information:

```bash
rusbit-cli info sample.torrent
```

## 📖 Usage

### Global Options

- <kbd>-v</kbd>, <kbd>--verbose</kbd>: Enable verbose logging
- <kbd>-p</kbd>, <kbd>--progress</kbd>: Show progress bar during downloads
- <kbd>-h</kbd>, <kbd>--help</kbd>: Display help information
- <kbd>-V</kbd>, <kbd>--version</kbd>: Display version information

### Commands

#### Decode Bencoded Value
```bash
rusbit-cli decode "<bencoded-string>"
```

#### Get Torrent Information
```bash
rusbit-cli info <torrent-file>
```

#### List Peers
```bash
rusbit-cli peers <torrent-file>
```

#### Perform Handshake
```bash
rusbit-cli handshake <torrent-file> <peer-address>
```

#### Download Single Piece
```bash
rusbit-cli download-piece -o <output-file> <torrent-file> <piece-index>
```

#### Download Complete File
```bash
rusbit-cli download -o <output-file> <torrent-file>
```

### Magnet Link Commands

<details>
<summary><strong>🔗 Magnet Link Operations</strong></summary>

#### Parse Magnet Link
```bash
rusbit-cli magnet-parse "<magnet-link>"
```

#### Magnet Handshake
```bash
rusbit-cli magnet-handshake "<magnet-link>"
```

#### Get Magnet Info
```bash
rusbit-cli magnet-info "<magnet-link>"
```

#### Download Piece via Magnet
```bash
rusbit-cli magnet-download-piece -o <output-file> "<magnet-link>" <piece-index>
```

#### Download Complete File via Magnet
```bash
rusbit-cli magnet-download -o <output-file> "<magnet-link>"
```

</details>

## 💡 Examples

### Basic Torrent Download
```bash
rusbit-cli download -o ubuntu.iso ubuntu.torrent
```

### Magnet Link Download with Verbose Output
```bash
rusbit-cli --verbose magnet-download -o movie.mp4 "magnet:?xt=urn:btih:...&dn=Movie&tr=..."
```

### Inspect Torrent Metadata
```bash
rusbit-cli info sample.torrent
```

Output:
```
Tracker URL: http://bittorrent-test-tracker.codecrafters.io/announce
File Name: sample.txt
File Length: 92063 bytes
Piece Length: 32768 bytes
Info Hash: 2d88a4bf3...
Piece Hashes:
  Piece 0: 1c8f1...
  Piece 1: 8e4f5...
  Piece 2: 3b2a7...
Total Pieces: 3
```

### Download with Progress Bar
```bash
rusbit-cli --progress download -o ubuntu.iso ubuntu.torrent
```

### Magnet Link Download with Progress Bar
```bash
rusbit-cli --progress magnet-download -o movie.mp4 "magnet:?xt=urn:btih:...&dn=Movie&tr=..."
```

## ⚙️ Configuration

On first run, Rusbit creates a `rusbit.toml` configuration file with default settings. You can modify this file to customize:

- **Peer ID prefix**: Customize your client identification
- **Listen port**: Port for incoming peer connections
- **Maximum connections**: Limit concurrent peer connections
- **Piece timeout**: Timeout for piece downloads (seconds)
- **Request timeout**: Timeout for peer requests (seconds)
- **Maximum retries**: Number of retry attempts for failed operations
- **Download directory**: Default output directory

Example `rusbit.toml`:
<details>
<summary><strong>📄 Example Configuration</strong></summary>

```toml
peer_id_prefix = "-RB0001-"
listen_port = 6881
max_connections = 50
piece_timeout = 30
request_timeout = 10
max_retries = 3
download_directory = "."
```

</details>

## 🔧 Troubleshooting

### Common Issues

<details>
<summary><strong>❌ Connection timeouts</strong></summary>
- Check your internet connection
- Try different torrent files or magnet links
- Adjust timeout settings in `rusbit.toml`
</details>

<details>
<summary><strong>🐌 Low download speeds</strong></summary>
- Ensure multiple peers are available
- Check firewall settings
- Try running with verbose logging: `rusbit-cli --verbose ...`
- Use the progress bar to monitor download status: `rusbit-cli --progress ...`
</details>

<details>
<summary><strong>🔨 Build failures</strong></summary>
- Ensure you have the latest Rust stable version
- Update dependencies: `cargo update`
- Clean and rebuild: `cargo clean && cargo build`
</details>

### Getting Help

- [Open an issue](https://github.com/nayiswftw/Rusbit/issues) on GitHub
- Check existing issues for similar problems
- Provide verbose logs when reporting bugs

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes and add tests
4. Run tests: `cargo test`
5. Commit your changes: `git commit -m 'Add amazing feature'`
6. Push to the branch: `git push origin feature/amazing-feature`
7. Open a Pull Request

### Development Setup

```bash
# Clone and build
git clone https://github.com/nayiswftw/Rusbit.git
cd rusbit-cli
cargo build

# Run tests
cargo test

# Run with sample data
cargo run -- info sample.torrent
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<div align="center">

**Made with ❤️ in Rust**

<p>
  <a href="https://github.com/nayiswftw/Rusbit">⭐ Star us on GitHub</a> •
  <a href="https://github.com/nayiswftw/Rusbit/issues">🐛 Report Issues</a>
</p>

</div>
