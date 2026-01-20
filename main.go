package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"text/template"

	"gopkg.in/yaml.v3"
)

// ANSI color codes
const (
	colorRed    = "\033[0;31m"
	colorGreen  = "\033[0;32m"
	colorYellow = "\033[1;33m"
	colorBlue   = "\033[0;34m"
	colorReset  = "\033[0m"
)

// Config represents the resume configuration
type Config struct {
	Sections []Section `yaml:"sections"`
	Output   Output    `yaml:"output"`
	Preamble string    `yaml:"preamble"`
}

// Section represents a resume section
type Section struct {
	Name     string `yaml:"name"`
	Enabled  bool   `yaml:"enabled"`
	Template string `yaml:"template"`
	Content  string `yaml:"content"`
}

// Output configuration
type Output struct {
	Directory string `yaml:"directory"`
	Filename  string `yaml:"filename"`
}

func main() {
	// Define command-line flags
	buildCmd := flag.Bool("build", false, "Generate LaTeX and compile to PDF")
	pdfCmd := flag.Bool("pdf", false, "Same as -build")
	cleanCmd := flag.Bool("clean", false, "Remove all generated files")
	validateCmd := flag.Bool("validate", false, "Check YAML files for syntax errors")
	showCmd := flag.Bool("show", false, "Display the generated LaTeX file")
	openCmd := flag.Bool("open", false, "Open the generated PDF")
	helpCmd := flag.Bool("help", false, "Show help message")
	
	flag.Parse()

	// Default behavior if no flags provided
	if !*buildCmd && !*pdfCmd && !*cleanCmd && !*validateCmd && !*showCmd && !*openCmd && !*helpCmd {
		*buildCmd = true
	}

	// Handle commands
	switch {
	case *helpCmd:
		printHelp()
	case *cleanCmd:
		cleanFiles()
	case *validateCmd:
		validateYAML()
	case *showCmd:
		showLaTeX()
	case *openCmd:
		openPDF()
	case *buildCmd || *pdfCmd:
		buildAndCompile()
	}
}

func buildResume() error {
	// Load configuration
	config, err := loadConfig("config.yaml")
	if err != nil {
		return fmt.Errorf("loading config: %w", err)
	}

	// Create output directory
	if err := os.MkdirAll(config.Output.Directory, 0755); err != nil {
		return fmt.Errorf("creating output directory: %w", err)
	}

	// Open output file
	outputPath := filepath.Join(config.Output.Directory, config.Output.Filename)
	outFile, err := os.Create(outputPath)
	if err != nil {
		return fmt.Errorf("creating output file: %w", err)
	}
	defer outFile.Close()

	// Write preamble
	preambleContent, err := os.ReadFile(config.Preamble)
	if err != nil {
		return fmt.Errorf("reading preamble: %w", err)
	}
	if _, err := outFile.Write(preambleContent); err != nil {
		return fmt.Errorf("writing preamble: %w", err)
	}

	// Start document
	if _, err := outFile.WriteString("\n\n\\begin{document}\n\n"); err != nil {
		return fmt.Errorf("writing document start: %w", err)
	}

	// Process each enabled section
	for _, section := range config.Sections {
		if !section.Enabled {
			fmt.Printf("⏭️  Skipping disabled section: %s\n", section.Name)
			continue
		}

		fmt.Printf("📝 Building section: %s\n", section.Name)

		// Load content data
		contentData, err := loadYAML(section.Content)
		if err != nil {
			return fmt.Errorf("loading content for %s: %w", section.Name, err)
		}

		// Load and execute template
		tmpl, err := template.ParseFiles(section.Template)
		if err != nil {
			return fmt.Errorf("parsing template for %s: %w", section.Name, err)
		}

		if err := tmpl.Execute(outFile, contentData); err != nil {
			return fmt.Errorf("executing template for %s: %w", section.Name, err)
		}

		if _, err := outFile.WriteString("\n\n"); err != nil {
			return fmt.Errorf("writing section separator: %w", err)
		}
	}

	// End document
	if _, err := outFile.WriteString("\\end{document}\n"); err != nil {
		return fmt.Errorf("writing document end: %w", err)
	}

	fmt.Printf("📄 Output written to: %s\n", outputPath)
	return nil
}

func loadConfig(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var config Config
	if err := yaml.Unmarshal(data, &config); err != nil {
		return nil, err
	}

	return &config, nil
}

func loadYAML(path string) (map[string]interface{}, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var content map[string]interface{}
	if err := yaml.Unmarshal(data, &content); err != nil {
		return nil, err
	}

	return content, nil
}

func printHelp() {
	fmt.Println("Resume Builder - Quick Commands")
	fmt.Println("")
	fmt.Println("Usage: go run main.go [flags]")
	fmt.Println("")
	fmt.Println("Flags:")
	fmt.Println("  -build      Generate LaTeX and compile to PDF (default)")
	fmt.Println("  -pdf        Same as -build")
	fmt.Println("  -clean      Remove all generated files")
	fmt.Println("  -validate   Check YAML files for syntax errors")
	fmt.Println("  -show       Display the generated LaTeX file")
	fmt.Println("  -open       Open the generated PDF")
	fmt.Println("  -help       Show this help message")
	fmt.Println("")
}

func buildAndCompile() {
	fmt.Printf("%s🔨 Building resume...%s\n", colorBlue, colorReset)
	
	if err := buildResume(); err != nil {
		log.Fatalf("%s❌ Error building resume: %v%s\n", colorRed, err, colorReset)
	}
	
	fmt.Printf("%s✅ LaTeX generated: output/resume.tex%s\n", colorGreen, colorReset)
	
	// Check if pdflatex is available
	if _, err := exec.LookPath("pdflatex"); err != nil {
		fmt.Printf("%s⚠️  pdflatex not found - skipping PDF compilation%s\n", colorYellow, colorReset)
		fmt.Printf("%s    Install texlive to enable PDF output (see INSTALL.md)%s\n", colorYellow, colorReset)
		return
	}
	
	fmt.Printf("%s📦 Compiling to PDF...%s\n", colorBlue, colorReset)
	
	if err := compilePDF(); err != nil {
		fmt.Printf("%s❌ PDF compilation failed%s\n", colorRed, colorReset)
		os.Exit(1)
	}
	
	fmt.Printf("%s✅ PDF generated: output/resume.pdf%s\n", colorGreen, colorReset)
}

func compilePDF() error {
	// Run pdflatex twice for references
	for i := 0; i < 2; i++ {
		cmd := exec.Command("pdflatex", "-interaction=nonstopmode", "resume.tex")
		cmd.Dir = "output"
		output, err := cmd.CombinedOutput()
		
		if err != nil {
			// Write log file
			logPath := filepath.Join("output", "pdflatex.log")
			os.WriteFile(logPath, output, 0644)
			
			// Print last 50 lines of output
			lines := splitLines(string(output))
			start := len(lines) - 50
			if start < 0 {
				start = 0
			}
			
			fmt.Printf("%s❌ PDF compilation failed. Error log:%s\n", colorRed, colorReset)
			for _, line := range lines[start:] {
				fmt.Println(line)
			}
			return err
		}
	}
	
	return nil
}

func cleanFiles() {
	fmt.Printf("%s🧹 Cleaning build artifacts...%s\n", colorYellow, colorReset)
	
	if err := os.RemoveAll("output"); err != nil {
		fmt.Printf("%s❌ Error cleaning files: %v%s\n", colorRed, err, colorReset)
		os.Exit(1)
	}
	
	fmt.Printf("%s✅ Clean complete%s\n", colorGreen, colorReset)
}

func validateYAML() {
	fmt.Printf("%s🔍 Validating YAML files...%s\n", colorBlue, colorReset)
	
	// Try to build to validate
	if err := buildResume(); err != nil {
		fmt.Printf("%s❌ YAML validation failed: %v%s\n", colorRed, err, colorReset)
		os.Exit(1)
	}
	
	fmt.Printf("%s✅ All YAML files appear valid%s\n", colorGreen, colorReset)
}

func showLaTeX() {
	content, err := os.ReadFile("output/resume.tex")
	if err != nil {
		fmt.Printf("%s❌ No resume.tex found. Run with -build first.%s\n", colorRed, colorReset)
		os.Exit(1)
	}
	
	fmt.Print(string(content))
}

func openPDF() {
	pdfPath := "output/resume.pdf"
	
	if _, err := os.Stat(pdfPath); os.IsNotExist(err) {
		fmt.Printf("%s❌ No resume.pdf found. Run with -build first.%s\n", colorRed, colorReset)
		os.Exit(1)
	}
	
	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "darwin":
		cmd = exec.Command("open", pdfPath)
	case "linux":
		cmd = exec.Command("xdg-open", pdfPath)
	case "windows":
		cmd = exec.Command("cmd", "/c", "start", pdfPath)
	default:
		fmt.Printf("%s❌ Unsupported platform for opening PDF%s\n", colorRed, colorReset)
		os.Exit(1)
	}
	
	if err := cmd.Start(); err != nil {
		fmt.Printf("%s❌ Error opening PDF: %v%s\n", colorRed, err, colorReset)
		os.Exit(1)
	}
	
	fmt.Printf("%s✅ Opening PDF...%s\n", colorGreen, colorReset)
}

func splitLines(s string) []string {
	var lines []string
	var line string
	
	for _, char := range s {
		if char == '\n' {
			lines = append(lines, line)
			line = ""
		} else {
			line += string(char)
		}
	}
	
	if line != "" {
		lines = append(lines, line)
	}
	
	return lines
}
