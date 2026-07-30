package main

import (
	"flag"
	"fmt"
	"log"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"strings"
	"text/template"
	"unicode"

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
	fmt.Println("  -open       Open the generated PDF in your default browser")
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

	config, err := loadConfig("config.yaml")
	if err != nil {
		fmt.Printf("%s⚠️  Could not load config to derive PDF name: %v%s\n", colorYellow, err, colorReset)
		fmt.Printf("%s✅ PDF generated: output/resume.pdf%s\n", colorGreen, colorReset)
		return
	}

	sourcePDF, targetPDF, err := derivePDFPaths(config)
	if err != nil {
		fmt.Printf("%s⚠️  Could not derive PDF name: %v%s\n", colorYellow, err, colorReset)
		fmt.Printf("%s✅ PDF generated: %s%s\n", colorGreen, sourcePDF, colorReset)
		return
	}

	if sourcePDF != targetPDF {
		if err := os.Rename(sourcePDF, targetPDF); err != nil {
			fmt.Printf("%s⚠️  Could not rename PDF: %v%s\n", colorYellow, err, colorReset)
			fmt.Printf("%s✅ PDF generated: %s%s\n", colorGreen, sourcePDF, colorReset)
			return
		}
	}

	fmt.Printf("%s✅ PDF generated: %s%s\n", colorGreen, targetPDF, colorReset)
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
	if config, err := loadConfig("config.yaml"); err == nil {
		if _, targetPDF, err := derivePDFPaths(config); err == nil {
			pdfPath = targetPDF
		}
	}

	if _, err := os.Stat(pdfPath); os.IsNotExist(err) {
		fmt.Printf("%s❌ No PDF found. Run with -build first.%s\n", colorRed, colorReset)
		os.Exit(1)
	}

	absPath, err := filepath.Abs(pdfPath)
	if err != nil {
		fmt.Printf("%s❌ Error resolving PDF path: %v%s\n", colorRed, err, colorReset)
		os.Exit(1)
	}

	fileURL := (&url.URL{Scheme: "file", Path: absPath}).String()
	if err := openFileInBrowser(fileURL); err != nil {
		fmt.Printf("%s❌ Error opening PDF in browser: %v%s\n", colorRed, err, colorReset)
		os.Exit(1)
	}

	fmt.Printf("%s✅ Opening PDF in browser...%s\n", colorGreen, colorReset)
}

func openFileInBrowser(fileURL string) error {
	var attempts []string

	if browser := strings.TrimSpace(os.Getenv("BROWSER")); browser != "" {
		if err := runBrowserSpec(browser, fileURL); err == nil {
			return nil
		} else {
			attempts = append(attempts, fmt.Sprintf("$BROWSER (%q): %v", browser, err))
		}
	}

	if runtime.GOOS == "linux" {
		if err := openWithXDGDefaultBrowser(fileURL); err == nil {
			return nil
		} else {
			attempts = append(attempts, fmt.Sprintf("xdg default browser: %v", err))
		}
	}

	for _, name := range []string{
		"firefox",
		"zen-beta",
		"zen",
		"zen-browser",
		"chromium",
		"chromium-browser",
		"google-chrome-stable",
		"google-chrome",
		"brave",
		"brave-browser",
		"microsoft-edge",
	} {
		if err := startCommand(name, fileURL); err == nil {
			return nil
		} else {
			attempts = append(attempts, fmt.Sprintf("%s: %v", name, err))
		}
	}

	switch runtime.GOOS {
	case "darwin":
		if err := startCommand("open", fileURL); err == nil {
			return nil
		} else {
			attempts = append(attempts, fmt.Sprintf("open: %v", err))
		}
	case "windows":
		if err := startCommand("cmd", "/c", "start", "", fileURL); err == nil {
			return nil
		} else {
			attempts = append(attempts, fmt.Sprintf("start: %v", err))
		}
	}

	if len(attempts) == 0 {
		return fmt.Errorf("no browser found; set $BROWSER or install a web browser")
	}

	return fmt.Errorf("%s", strings.Join(attempts, "; "))
}

func runBrowserSpec(browser, fileURL string) error {
	if strings.Contains(browser, "%s") {
		command := strings.ReplaceAll(browser, "%s", shellQuote(fileURL))
		return exec.Command("sh", "-c", command).Start()
	}

	parts := strings.Fields(browser)
	if len(parts) == 0 {
		return fmt.Errorf("empty $BROWSER")
	}

	return startCommand(parts[0], append(parts[1:], fileURL)...)
}

func openWithXDGDefaultBrowser(fileURL string) error {
	out, err := exec.Command("xdg-settings", "get", "default-web-browser").Output()
	if err != nil {
		return err
	}

	desktopFile := strings.TrimSpace(string(out))
	if desktopFile == "" {
		return fmt.Errorf("no default web browser configured")
	}

	desktopPath, err := findDesktopFile(desktopFile)
	if err != nil {
		return err
	}

	args, err := desktopExecCommand(desktopPath, fileURL)
	if err != nil {
		return err
	}
	if len(args) == 0 {
		return fmt.Errorf("empty Exec in %s", desktopFile)
	}

	return startCommand(args[0], args[1:]...)
}

func startCommand(command string, args ...string) error {
	if !filepath.IsAbs(command) {
		path, err := exec.LookPath(command)
		if err != nil {
			return err
		}
		command = path
	}

	return exec.Command(command, args...).Start()
}

func findDesktopFile(name string) (string, error) {
	searchDirs := []string{}

	if home, err := os.UserHomeDir(); err == nil {
		searchDirs = append(searchDirs, filepath.Join(home, ".local/share/applications"))
	}

	if dataDirs := os.Getenv("XDG_DATA_DIRS"); dataDirs != "" {
		for _, dir := range strings.Split(dataDirs, ":") {
			searchDirs = append(searchDirs, filepath.Join(dir, "applications"))
		}
	} else {
		searchDirs = append(searchDirs, "/usr/local/share/applications", "/usr/share/applications")
	}

	for _, dir := range searchDirs {
		candidate := filepath.Join(dir, name)
		if _, err := os.Stat(candidate); err == nil {
			return candidate, nil
		}
	}

	return "", fmt.Errorf("desktop file not found: %s", name)
}

func desktopExecCommand(desktopPath, fileURL string) ([]string, error) {
	data, err := os.ReadFile(desktopPath)
	if err != nil {
		return nil, err
	}

	for _, line := range strings.Split(string(data), "\n") {
		if !strings.HasPrefix(line, "Exec=") {
			continue
		}

		execLine := strings.TrimSpace(strings.TrimPrefix(line, "Exec="))
		execLine = expandDesktopExec(execLine, fileURL)
		if execLine == "" {
			continue
		}

		return strings.Fields(execLine), nil
	}

	return nil, fmt.Errorf("no Exec= line in %s", desktopPath)
}

var desktopFieldCodeRE = regexp.MustCompile(`%[a-zA-Z]`)

func expandDesktopExec(execLine, fileURL string) string {
	replacer := strings.NewReplacer(
		"%u", fileURL,
		"%U", fileURL,
		"%f", fileURL,
		"%F", fileURL,
	)
	execLine = replacer.Replace(execLine)
	execLine = desktopFieldCodeRE.ReplaceAllString(execLine, "")
	return strings.TrimSpace(execLine)
}

func shellQuote(value string) string {
	if value == "" {
		return "''"
	}

	return "'" + strings.ReplaceAll(value, "'", `'\'"\'"\'"`) + "'"
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

func derivePDFPaths(config *Config) (string, string, error) {
	baseName := strings.TrimSuffix(config.Output.Filename, filepath.Ext(config.Output.Filename))
	if baseName == "" {
		baseName = "resume"
	}

	sourcePDF := filepath.Join(config.Output.Directory, baseName+".pdf")

	headerName, err := getHeaderName(config)
	if err != nil {
		return sourcePDF, sourcePDF, err
	}

	slug := slugifyName(headerName)
	if slug == "" {
		return sourcePDF, sourcePDF, nil
	}

	targetPDF := filepath.Join(config.Output.Directory, fmt.Sprintf("%s-%s.pdf", slug, baseName))
	return sourcePDF, targetPDF, nil
}

func getHeaderName(config *Config) (string, error) {
	var headerPath string
	for _, section := range config.Sections {
		if strings.EqualFold(section.Name, "header") {
			headerPath = section.Content
			break
		}
	}

	if headerPath == "" {
		return "", nil
	}

	content, err := loadYAML(headerPath)
	if err != nil {
		return "", err
	}

	if nameValue, ok := content["name"]; ok {
		if name, ok := nameValue.(string); ok {
			return name, nil
		}
	}

	return "", nil
}

func slugifyName(input string) string {
	lower := strings.ToLower(strings.TrimSpace(input))
	if lower == "" {
		return ""
	}

	var b strings.Builder
	lastDash := false

	for _, r := range lower {
		if unicode.IsLetter(r) || unicode.IsDigit(r) {
			b.WriteRune(r)
			lastDash = false
			continue
		}

		if !lastDash {
			b.WriteRune('-')
			lastDash = true
		}
	}

	slug := strings.Trim(b.String(), "-")
	return slug
}
