# Contributing to OpenFlash

First off, thanks for taking the time to contribute! 🎉

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How Can I Contribute?

### 🐛 Reporting Bugs

Before creating bug reports, please check existing issues. When creating a bug report, include:

- **Clear title** describing the issue
- **Steps to reproduce** the behavior
- **Expected behavior** vs what actually happened
- **Screenshots** if applicable
- **Environment info**: OS, hardware (RP2040/STM32), NAND chip model

Use the bug report template when opening an issue.

### 💡 Suggesting Features

Feature requests are welcome! Please:

- Check if the feature was already requested
- Describe the use case clearly
- Explain why this would be useful to most users

### 🔧 Pull Requests

1. **Fork** the repository
2. **Create a branch** from `main`: `git checkout -b feature/amazing-feature`
3. **Make your changes** following our coding standards
4. **Test** your changes thoroughly
5. **Commit** with clear messages: `git commit -m 'Add amazing feature'`
6. **Push** to your fork: `git push origin feature/amazing-feature`
7. **Open a Pull Request**

## Development Setup

### Prerequisites

- Rust 1.70+ (`rustup update stable`)
- Node.js 18+ 
- Tauri CLI (`cargo install tauri-cli`)

### Building

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/openflash.git
cd openflash/openflash

# Install frontend dependencies
cd gui && npm install && cd ..

# Run in development mode
cargo tauri dev

# Run tests
cargo test -p openflash-core
```

### Project Structure

```
openflash/
├── core/           # Shared Rust library (ECC, ONFI, analysis)
├── gui/            # Tauri desktop app
│   ├── src/        # React frontend
│   └── src-tauri/  # Rust backend
└── firmware/       # MCU firmware (RP2040, STM32F1)
```

## Coding Standards

### Rust

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add tests for new functionality
- Document public APIs with doc comments

### TypeScript/React

- Use functional components with hooks
- Follow existing code style
- Use TypeScript strictly (no `any`)
- Keep components small and focused

### Commits

- Use clear, descriptive commit messages
- Reference issues when applicable: `Fix #123`
- Keep commits atomic (one logical change per commit)

## Testing

### Core Library
```bash
cargo test -p openflash-core
```

### GUI (with mock device)
1. Run `cargo tauri dev`
2. Click "Mock" to enable mock device
3. Test all operations

### Firmware
Firmware requires actual hardware for testing. Document your test setup in PRs.

## Areas We Need Help

- 🔌 **Hardware**: Testing with different NAND chips
- 📝 **Documentation**: Tutorials, wiki pages
- 🌍 **Translations**: UI localization
- 🧪 **Testing**: Edge cases, stress testing
- 🎨 **Design**: UI/UX improvements
- 🔧 **Features**: See [Issues](https://github.com/openflash/openflash/issues)

## Recognition

Contributors are recognized in:
- README.md contributors section
- Release notes
- GitHub contributors page

## Questions?

- Open a [Discussion](https://github.com/openflash/openflash/discussions)
- Join our community chat (coming soon)

Thank you for contributing! 🔥
