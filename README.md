# Resume Builder

A modular LaTeX resume build system that separates content from formatting resulting in a highly configurable beautiful resume.

## 🎯 Philosophy

This system breaks your resume into content 'blocks' each with:
- **Content** (YAML files) contains your actual resume data
- **Templates** (LaTeX files) define how sections are formatted
- **Configuration** (config.yaml) controls which sections to include
- **Build system** (Go) assembles everything into a final LaTeX document

## 📁 Project Structure

```
resume/
├── content/              # Your resume data edit to update your resume (YAML files)
│   ├── header.yaml      # Contact info
│   ├── education.yaml   # Education history
│   ├── experience.yaml  # Work experience
│   ├── projects.yaml    # Projects
│   └── skills.yaml      # Skills and certifications
├── templates/           # LaTeX formatting templates
│   ├── preamble.tex    # LaTeX preamble and commands
│   ├── header.tex      # Header section template
│   ├── education.tex   # Education section template
│   ├── experience.tex  # Experience section template
│   ├── projects.tex    # Projects section template
│   └── skills.tex      # Skills section template
├── output/             # Generated files (git-ignored)
│   ├── resume.tex      # Generated LaTeX file
│   └── resume.pdf      # Compiled PDF
├── config.yaml         # Build configuration
├── main.go            # Build system
├── build.sh           # Build commands
└── README.md          # This file
```

## 🚀 Quick Start

### Prerequisites
- Go 1.21 or higher
- pdflatex (for PDF compilation)

### Building Your Resume

1. **Install dependencies:**
   ```bash
   go mod download
   ```

2. **Generate LaTeX file:**
   ```bash
   go run main.go
   ```

3. **Compile to PDF:**
   ```bash
   go run main.go
   ```

4. **Clean build artifacts:**
   ```bash
   go run main.go -clean
   ```

## 📝 Usage

### Adding/Removing Sections

Edit `config.yaml` to control which sections appear:

```yaml
sections:
  - name: "education"
    enabled: true        # Set to false to exclude this section
    template: "templates/education.tex"
    content: "content/education.yaml"
```

Sections are rendered in the order they appear in the config file.

### Editing Content

Simply edit the YAML files in `content/`:

**Example - Adding a new project:**

```yaml
# content/projects.yaml
projects:
  - name: "My New Project"
    technologies: "Go, Docker, Kubernetes"
    date: "Jan. 2026 -- Present"
    details:
      - "Built a scalable microservices architecture"
      - "Implemented CI/CD pipeline with GitHub Actions"
```

### Customizing Templates

Modify the LaTeX templates in `templates/` to change formatting. Templates use Go's `text/template` syntax:

```latex
{{- range .Projects}}
  \resumeProjectHeading
    {\textbf{ {{- .Name -}} }}{ {{- .Date -}} }
{{- end}}
```

### Creating New Sections

1. Create content file: `content/newsection.yaml`
2. Create template file: `templates/newsection.tex`
3. Add to `config.yaml`:
   ```yaml
   sections:
     - name: "newsection"
       enabled: true
       template: "templates/newsection.tex"
       content: "content/newsection.yaml"
   ```

## 🎨 Customization

### Changing LaTeX Styles

Edit `templates/preamble.tex` to modify:
- Document class and options
- Margins and spacing
- Custom commands
- Fonts and colors

### Content Fields

Each content YAML file has its own structure. See existing files for examples:
- `header.yaml` - name, phone, email, links
- `education.yaml` - degree, date, school
- `experience.yaml` - position, date, company, highlights
- `projects.yaml` - name, technologies, date, details
- `skills.yaml` - various skill categories

## 🔧 Advanced Usage

### Multiple Resume Versions

Create different config files for different audiences:

```bash
# config-full.yaml - Complete resume
# config-short.yaml - One-page version
# config-academic.yaml - Academic focus

go run main.go -config config-short.yaml
```

### Adding Conditional Content

Use YAML anchors and aliases to reuse content:

```yaml
common: &security-skills
  - "Penetration testing"
  - "Vulnerability assessment"

skills:
  securityTools: *security-skills
```

## 📚 Benefits

✅ **Modular** - Add/remove sections easily
✅ **Version Control** - Track changes to content separately from formatting
✅ **Reusable** - Same content, multiple formats
✅ **Maintainable** - Clear separation of concerns
✅ **Flexible** - Easy to customize for different job applications

## 🛠️ Troubleshooting

**Issue: PDF compilation fails**
- Ensure pdflatex is installed: `pdflatex --version`
- Check `output/resume.log` for LaTeX errors

**Issue: Template parsing errors**
- Verify YAML syntax in content files
- Check template syntax matches Go text/template format

**Issue: Missing sections**
- Verify `enabled: true` in config.yaml
- Check file paths in config.yaml are correct

---

**Author:** Cullen Kelley
