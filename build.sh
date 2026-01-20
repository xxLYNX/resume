#!/bin/bash
# Quick helper script for common resume operations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
print_help() {
    echo "Resume Builder - Quick Commands"
    echo ""
    echo "Usage: ./build.sh [command]"
    echo ""
    echo "Commands:"
    echo "  build      - Generate LaTeX and compile to PDF (auto-detects pdflatex)"
    echo "  pdf        - Same as build (kept for compatibility)"
    echo "  clean      - Remove all generated files"
    echo "  validate   - Check YAML files for syntax errors"
    echo "  show       - Display the generated LaTeX file"
    echo "  open       - Open the generated PDF (if it exists)"
    echo "  help       - Show this help message"
    echo ""
}

build_resume() {
    echo -e "${BLUE}🔨 Building resume...${NC}"
    go run main.go
    echo -e "${GREEN}✅ LaTeX generated: output/resume.tex${NC}"
    
    # Auto-compile to PDF if pdflatex is available
    if command -v pdflatex &> /dev/null; then
        echo -e "${BLUE}📦 Compiling to PDF...${NC}"
        cd output
        if pdflatex -interaction=nonstopmode resume.tex > pdflatex.log 2>&1; then
            # Run twice for references
            pdflatex -interaction=nonstopmode resume.tex > pdflatex.log 2>&1
            cd ..
            echo -e "${GREEN}✅ PDF generated: output/resume.pdf${NC}"
        else
            cd ..
            echo -e "${RED}❌ PDF compilation failed. Error log:${NC}"
            tail -50 output/pdflatex.log
            exit 1
        fi
    else
        echo -e "${YELLOW}⚠️  pdflatex not found - skipping PDF compilation${NC}"
        echo -e "${YELLOW}    Install texlive to enable PDF output (see INSTALL.md)${NC}"
    fi
}

compile_pdf() {
    if ! command -v pdflatex &> /dev/null; then
        echo -e "${RED}❌ Error: pdflatex not found${NC}"
        echo "Install a LaTeX distribution to compile PDFs."
        echo "See INSTALL.md for instructions."
        exit 1
    fi
    
    build_resume
    echo -e "${BLUE}📦 Compiling to PDF...${NC}"
    cd output
    if pdflatex -interaction=nonstopmode resume.tex > pdflatex.log 2>&1; then
        pdflatex -interaction=nonstopmode resume.tex > pdflatex.log 2>&1
        cd ..
        echo -e "${GREEN}✅ PDF generated: output/resume.pdf${NC}"
    else
        cd ..
        echo -e "${RED}❌ PDF compilation failed. Error log:${NC}"
        tail -50 output/pdflatex.log
        exit 1
    fi
}

clean_files() {
    echo -e "${YELLOW}🧹 Cleaning build artifacts...${NC}"
    rm -rf output/
    echo -e "${GREEN}✅ Clean complete${NC}"
}

validate_yaml() {
    echo -e "${BLUE}🔍 Validating YAML files...${NC}"
    
    # Check if yq is installed for validation
    if command -v yq &> /dev/null; then
        for file in config.yaml content/*.yaml; do
            if yq eval '.' "$file" > /dev/null 2>&1; then
                echo -e "${GREEN}✓${NC} $file"
            else
                echo -e "${RED}✗${NC} $file - Invalid YAML"
            fi
        done
    else
        # Fallback: try to build to validate
        echo "yq not installed, attempting build to validate..."
        if go run main.go > /dev/null 2>&1; then
            echo -e "${GREEN}✅ All YAML files appear valid${NC}"
        else
            echo -e "${RED}❌ YAML validation failed${NC}"
            exit 1
        fi
    fi
}

show_latex() {
    if [ -f output/resume.tex ]; then
        less output/resume.tex
    else
        echo -e "${RED}❌ No resume.tex found. Run 'build' first.${NC}"
        exit 1
    fi
}

open_pdf() {
    if [ -f output/resume.pdf ]; then
        if command -v xdg-open &> /dev/null; then
            xdg-open output/resume.pdf
        elif command -v open &> /dev/null; then
            open output/resume.pdf
        else
            echo -e "${YELLOW}Cannot automatically open PDF. File: output/resume.pdf${NC}"
        fi
    else
        echo -e "${RED}❌ No resume.pdf found. Run 'pdf' first.${NC}"
        exit 1
    fi
}

# Main logic
case "${1:-build}" in
    build)
        build_resume
        ;;
    pdf|compile)
        compile_pdf
        ;;
    clean)
        clean_files
        ;;
    validate)
        validate_yaml
        ;;
    show)
        show_latex
        ;;
    open)
        open_pdf
        ;;
    help|--help|-h)
        print_help
        ;;
    *)
        echo -e "${RED}Unknown command: $1${NC}"
        echo ""
        print_help
        exit 1
        ;;
esac
