package main

import (
	"fmt"
	"log"
	"os"
	"path/filepath"
	"text/template"

	"gopkg.in/yaml.v3"
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
	if err := buildResume(); err != nil {
		log.Fatalf("Error building resume: %v", err)
	}
	fmt.Println("✅ Resume built successfully!")
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
