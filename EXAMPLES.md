# Examples: Customizing Your Resume

## Example 1: Creating a Short Version (One-Page Resume)

Create a new config file `config-short.yaml`:

```yaml
sections:
  - name: "header"
    enabled: true
    template: "templates/header.tex"
    content: "content/header.yaml"
    
  - name: "education"
    enabled: true
    template: "templates/education.tex"
    content: "content/education.yaml"
    
  - name: "experience"
    enabled: true
    template: "templates/experience.tex"
    content: "content/experience-short.yaml"  # Use shortened version
    
  - name: "skills"
    enabled: true
    template: "templates/skills.tex"
    content: "content/skills.yaml"

output:
  directory: "output"
  filename: "resume-short.tex"
  
preamble: "templates/preamble.tex"
```

Create `content/experience-short.yaml` with only your most recent positions.

## Example 2: Academic Resume

Disable work experience, emphasize projects and education:

```yaml
sections:
  - name: "header"
    enabled: true
  - name: "education"
    enabled: true
  - name: "experience"
    enabled: false      # Disable work experience
  - name: "projects"
    enabled: true
  - name: "skills"
    enabled: true
```

## Example 3: Adding a Publications Section

1. Create `content/publications.yaml`:

```yaml
publications:
  - title: "Malware Classification Using Machine Learning"
    venue: "IEEE Conference on Security"
    date: "Dec. 2024"
    authors: "C. Kelley, K. Sun"
    link: "https://example.com/paper.pdf"
```

2. Create `templates/publications.tex`:

```latex
{{- /* Publications section template */ -}}
\section{Publications}
    \resumeSubHeadingListStart
{{- range .publications}}
      \resumeProjectHeading
          {\textbf{ {{- .title -}} }}{ {{- .date -}} }
          \resumeItemListStart
            \resumeItem{ {{- .venue -}} }
            \resumeItem{Authors: {{.authors}}}
          \resumeItemListEnd
{{- end}}
    \resumeSubHeadingListEnd
```

3. Add to `config.yaml`:

```yaml
sections:
  # ... other sections
  - name: "publications"
    enabled: true
    template: "templates/publications.tex"
    content: "content/publications.yaml"
```

## Example 4: Multiple Resume Variants

**Software Security Focus:**
```bash
# config-security.yaml - emphasizes security projects and skills
go run main.go  # Uses default config.yaml
```

**Software Engineering Focus:**
```bash
# config-engineering.yaml - emphasizes development projects
# Disable security-specific projects, add more engineering projects
```

## Example 5: Conditional Content with YAML Features

Use different experience descriptions for different audiences:

```yaml
# content/experience.yaml
experience:
  - position: "Software Security Consultant"
    date: "Jun. 2024 -- Aug. 2024"
    company: "Affinity DB | Leesburg, VA"
    highlights_technical: &technical  # Define anchor
      - "Performed static analysis on 50+ codebases"
      - "Identified 23 critical vulnerabilities"
      - "Utilized IDA Pro, Ghidra, and custom tools"
    highlights_business: &business    # Define anchor
      - "Improved security posture for client applications"
      - "Delivered comprehensive security reports"
      - "Collaborated with stakeholders"
    highlights: *technical  # Use technical by default
```

Then create different content files:
- `content/experience-technical.yaml` (uses `*technical`)
- `content/experience-business.yaml` (uses `*business`)

## Example 6: Template Customization - Different Formatting

Create `templates/experience-detailed.tex` with more spacing:

```latex
{{- /* Detailed experience template with more spacing */ -}}
\section{Professional Experience}
  \resumeSubHeadingListStart
{{- range .experience}}

    \vspace{3pt}  % Extra spacing
    \resumeSubheading
      {\Large {{- .position -}} }{ {{- .date -}} }
      { {{- .company -}} }{}
      
    \textbf{Key Contributions:}
    \resumeItemListStart
{{- range .highlights}}
        \resumeItem{ {{- . -}} }
{{- end}}
      \resumeItemListEnd
{{- end}}

  \resumeSubHeadingListEnd
```

## Example 7: Quick Section Reordering

Want to put Projects before Experience? Just reorder in config.yaml:

```yaml
sections:
  - name: "header"
    enabled: true
  - name: "education"
    enabled: true
  - name: "projects"      # Projects first
    enabled: true
  - name: "experience"    # Experience second
    enabled: true
  - name: "skills"
    enabled: true
```

## Building Custom Versions

```bash
# Default full resume
make build

# Short version
# Modify config.yaml to disable some sections, then:
make build

# Or use multiple config files (requires modifying main.go to accept flags)
```

## Tips

1. **Version Control Different Configs**: Keep different config files for different job types
2. **Reuse Content**: Use YAML anchors to avoid duplication
3. **Test Incrementally**: Build after each change to catch errors early
4. **Keep Templates Simple**: Let Go handle logic, keep LaTeX focused on formatting
5. **Comment Your YAML**: Use `# comments` to document why certain content exists
