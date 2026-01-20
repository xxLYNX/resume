# Installation & Setup Guide

## Prerequisites

### Required
- **Go 1.19+**: [Download Go](https://go.dev/dl/)
  ```bash
  go version  # Verify installation
  ```

### Optional (for PDF compilation)
- **LaTeX Distribution**: Required only if you want to compile to PDF

#### Linux (Debian/Ubuntu)
```bash
sudo apt-get update
sudo apt-get install texlive-latex-base texlive-latex-extra
```

#### Linux (Fedora/RHEL)
```bash
sudo dnf install texlive texlive-collection-latexextra
```

#### macOS
```bash
brew install --cask mactex
# Or for a smaller installation:
brew install --cask basictex
```

#### Windows
Download and install [MiKTeX](https://miktex.org/download) or [TeX Live](https://tug.org/texlive/windows.html)

## Setup

1. **Clone or initialize the repository**
   ```bash
   cd ~/Projects/resume
   ```

2. **Install Go dependencies**
   ```bash
   go mod download
   ```

3. **Build your resume**
   ```bash
   go run main.go
   ```

4. **View the generated LaTeX**
   ```bash
   cat output/resume.tex
   ```

5. **Compile to PDF** (optional, requires pdflatex)
   ```bash
   go run main.go
   ```

## Workflow

### Basic Workflow
1. Edit content files in `content/`
2. Run `go run main.go` to generate LaTeX and PDF
3. Check `output/resume.tex` for correctness and `output/resume.pdf` for final output

### Advanced Workflow
1. Modify templates in `templates/` to change formatting
2. Edit `config.yaml` to enable/disable sections
3. Create custom content files for different resume versions
4. Build and review

## Troubleshooting

### Go Module Issues
```bash
# If you see module-related errors:
go mod tidy
go mod download
```

### LaTeX Compilation Errors
```bash
# Check the LaTeX log file:
cat output/resume.log

# Common issues:
# - Missing packages: Install additional LaTeX packages
# - Syntax errors: Check the generated .tex file
# - Encoding issues: Ensure YAML files are UTF-8
```

### Template Errors
```bash
# If the build fails with template errors:
# 1. Check YAML syntax (use a YAML validator)
# 2. Ensure field names match between content and templates
# 3. Check for missing required fields
```

## Alternative LaTeX Compilation

If you don't want to install LaTeX locally, you can:

1. **Use Overleaf**
   - Copy `output/resume.tex` to [Overleaf](https://www.overleaf.com)
   - Compile online

2. **Use Docker**
   ```bash
   docker run --rm -v "$PWD/output:/data" texlive/texlive pdflatex /data/resume.tex
   ```

3. **Use CI/CD**
   - Set up GitHub Actions to compile LaTeX
   - Automatically generate PDFs on commit

## Verification

To verify everything is working:

```bash
# 1. Check Go is installed
go version

# 2. Build the resume
make build

# 3. Verify output exists
ls -lh output/resume.tex

# 4. Check the file has content
wc -l output/resume.tex  # Should show ~180+ lines

# 5. If pdflatex is installed:
make pdf
ls -lh output/resume.pdf
```

## Next Steps

1. **Customize your content**: Edit files in `content/`
2. **Adjust formatting**: Modify templates in `templates/`
3. **Create variants**: Copy `config.yaml` to create different resume versions
4. **Read examples**: Check `EXAMPLES.md` for advanced usage patterns

## Getting Help

- Check the LaTeX log: `output/resume.log` (after attempting PDF compilation)
- Validate YAML: Use online YAML validators
- Review examples: See `EXAMPLES.md`
- Check README: See `README.md` for architecture overview
