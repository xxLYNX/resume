# Quick Start Guide

Welcome to your modular LaTeX resume build system! 🚀

## What You Have

A Hugo-style static site generator for your resume:
- **Content** in YAML files (easy to edit)
- **Templates** in LaTeX (formatting)
- **Build system** in Go (assembles everything)

## Getting Started (3 Steps)

1. **Edit your content** (modify YAML files in `content/`)
   ```bash
   vim content/experience.yaml
   ```

2. **Build your resume**
   ```bash
   go run main.go
   ```

3. **View the output**
   ```bash
   cat output/resume.tex
   ```

## Common Tasks

### Add a New Job
Edit [content/experience.yaml](content/experience.yaml):
```yaml
experience:
  - position: "New Job Title"
    date: "Jan. 2026 -- Present"
    company: "Company Name | Location"
    highlights:
      - "Achievement 1"
      - "Achievement 2"
```

### Add a New Project
Edit [content/projects.yaml](content/projects.yaml):
```yaml
projects:
  - name: "Project Name"
    technologies: "Go, Python, Docker"
    date: "Jan. 2026 -- Present"
    details:
      - "What you built"
      - "What you achieved"
```

### Remove a Section
Edit [config.yaml](config.yaml):
```yaml
sections:
  - name: "projects"
    enabled: false    # Just set to false!
```

### Compile to PDF
```bash
go run main.go
# Requires pdflatex - see INSTALL.md
```

## Project Structure

```
resume/
├── content/          ← Your resume data (edit these!)
│   ├── header.yaml
│   ├── education.yaml
│   ├── experience.yaml
│   ├── projects.yaml
│   └── skills.yaml
├── templates/        ← LaTeX formatting (rarely change)
├── config.yaml       ← Control which sections appear
├── main.go          ← Build system (don't touch)
└── README.md        ← Project docs
```

## Commands Reference
```bash
go run main.go           # Generate LaTeX and PDF (default)
go run main.go -pdf      # Same as above
go run main.go -clean    # Remove output files
go run main.go -validate # Check YAML syntax
go run main.go -show     # View generated LaTeX
go run main.go -open     # Open PDF in viewer
```

## Workflow Example

```bash
# 1. Make your changes
vim content/experience.yaml

# 2. Build
go run main.go

# 3. Check output
cat output/resume.tex

# 4. Open the PDF
go run main.go -open
```

## Tips

✅ **Edit content, not LaTeX** - Keep your changes in YAML files  
✅ **Use version control** - Commit your content changes  
✅ **Create variants** - Copy config.yaml for different resume versions  
✅ **Test frequently** - Run `go run main.go` after each change  
✅ **Read the examples** - See [EXAMPLES.md](EXAMPLES.md) for advanced usage  

## Need Help?

- **Installation issues**: See [INSTALL.md](INSTALL.md)
- **How to customize**: See [EXAMPLES.md](EXAMPLES.md)
- **Architecture details**: See [README.md](README.md)
- **Build errors**: Check `output/resume.log` (after PDF compilation)

## Key Benefits

🎯 **Modular**: Add/remove sections in seconds  
📝 **Version Control Friendly**: Track content changes separately  
🔄 **Reusable**: Same content, multiple formats  
⚡ **Fast**: Instant builds with Go  
🎨 **Flexible**: Easy to customize for different applications  

---

**You're all set!** Start by editing `content/experience.yaml` and run `go run main.go`.
