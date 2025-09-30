<div align="center" style="background-color: #584c4cff; padding: 20px; border-radius: 10px;">
<h1 style="color: #ec7e7eff;">Rusbit</h1>

A command-line BitTorrent client written in Rust. Supports downloading torrents from `.torrent` files and magnet links.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Cargo](https://img.shields.io/badge/cargo-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://doc.rust-lang.org/cargo/)

</div>




<div align="center">
    <h2>Features</h2>
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

## Installation 🚀

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)

### Build from Source

```bash
git clone <repository-url>
cd rusbit-cli
cargo build --release
```

The binary will be available at `target/release/rusbit-cli`.

## Usage 📖

### Basic Commands

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
rusbit-cli download-piece <output-file> <torrent-file> <piece-index>
```

#### Download Complete File
```bash
rusbit-cli download <output-file> <torrent-file>
```

### Magnet Link Commands

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
rusbit-cli magnet-download-piece <output-file> "<magnet-link>" <piece-index>
```

#### Download Complete File via Magnet
```bash
rusbit-cli magnet-download <output-file> "<magnet-link>"
```

## Examples 💡

### Download a Torrent File
```bash
rusbit-cli download myfile.txt sample.torrent
```

### Download via Magnet Link
```bash
rusbit-cli magnet-download myfile.txt "magnet:?xt=urn:btih:...&dn=MyFile&tr=..."
```

### Get Torrent Information
```bash
rusbit-cli info sample.torrent
```

Output:
```
Tracker URL: http://example.com/announce
File Name: sample.txt
File Length: 123456 bytes
Piece Length: 16384 bytes
Info Hash: abc123...
Piece Hashes:
  Piece 0: abc123...
  Piece 1: def456...
  ...
Total Pieces: 8
```


## Contributing 🤝

Contributions are welcome! Please feel free to submit a Pull Request.

## License 📄

This project is licensed under the MIT License - see the LICENSE file for details.
