#!/usr/bin/env python3
"""
Comprehensive Symbol → Instrument Migration Script
=================================================

This script performs a complete migration from "symbol" terminology to "instrument" 
terminology throughout the entire AlphaPulse codebase. It handles:

1. Rust code: structs, functions, variables, comments, documentation
2. Database schema: table names, column names, indexes
3. Configuration files: YAML configs, environment variables
4. Frontend code: JavaScript/TypeScript variables, API endpoints
5. Documentation: README files, comments, API docs

Usage:
    python migrate_symbol_to_instrument.py --dry-run    # Preview changes
    python migrate_symbol_to_instrument.py --execute    # Apply changes

The script creates backups before making changes and provides detailed logging.
"""

import os
import re
import shutil
import argparse
import json
from pathlib import Path
from typing import List, Dict, Tuple, Set
import logging

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class SymbolToInstrumentMigrator:
    def __init__(self, dry_run: bool = True):
        self.dry_run = dry_run
        self.backend_root = Path("/Users/daws/alphapulse/backend")
        self.frontend_root = Path("/Users/daws/alphapulse/frontend") 
        self.docs_root = Path("/Users/daws/alphapulse/docs")
        self.changes = []
        self.backup_dir = Path("/Users/daws/alphapulse/migration_backup")
        
        # Comprehensive mapping of symbol → instrument transformations
        self.transformations = [
            # Rust structs and types
            ("SymbolDescriptor", "InstrumentDescriptor"),
            ("SymbolMappingMessage", "InstrumentMappingMessage"),
            ("SymbolMapping", "InstrumentMapping"),
            ("SymbolHash", "InstrumentHash"),
            ("SymbolRegistry", "InstrumentRegistry"),
            
            # Function names
            ("normalize_symbol", "normalize_instrument"),
            ("parse_symbol_descriptor", "parse_instrument_descriptor"),
            ("parse_symbol", "parse_instrument"),
            ("format_symbol", "format_instrument"),
            ("hash_symbol", "hash_instrument"),
            ("get_symbol", "get_instrument"),
            ("set_symbol", "set_instrument"),
            ("create_symbol", "create_instrument"),
            ("register_symbol", "register_instrument"),
            ("lookup_symbol", "lookup_instrument"),
            
            # Variables and fields
            ("symbol_hash", "instrument_hash"),
            ("symbol_to_hash", "instrument_to_hash"),
            ("hash_to_symbol", "hash_to_instrument"),
            ("symbol_mapping", "instrument_mapping"),
            ("symbol_descriptor", "instrument_descriptor"),
            ("symbol_registry", "instrument_registry"),
            ("symbol_data", "instrument_data"),
            ("symbol_info", "instrument_info"),
            ("symbol_config", "instrument_config"),
            ("symbol_cache", "instrument_cache"),
            ("symbols", "instruments"),
            ("symbol", "instrument"),
            
            # Database fields and tables
            ("symbol_hash", "instrument_hash"),
            ("symbol", "instrument"),
            ("symbol_mapping", "instrument_mapping"),
            
            # Constants and enums
            ("SYMBOL_", "INSTRUMENT_"),
            ("Symbol::", "Instrument::"),
            
            # Configuration keys
            ("symbol_registry", "instrument_registry"),
            ("symbol_cache", "instrument_cache"),
            
            # API endpoints and JSON fields
            ("/symbol/", "/instrument/"),
            ("/symbols/", "/instruments/"),
            ("\"symbol\":", "\"instrument\":"),
            ("'symbol':", "'instrument':"),
            ("symbol_hash", "instrument_hash"),
            
            # Comments and documentation
            ("symbol hash", "instrument hash"),
            ("symbol mapping", "instrument mapping"),
            ("symbol descriptor", "instrument descriptor"),
            ("symbol registry", "instrument registry"),
            ("Symbol hash", "Instrument hash"),
            ("Symbol mapping", "Instrument mapping"),
            ("Symbol descriptor", "Instrument descriptor"),
            ("Symbol registry", "Instrument registry"),
            
            # Case variations for text/comments
            ("symbol", "instrument"),  # This will catch most remaining cases
            ("Symbol", "Instrument"),  # Capitalized versions
        ]
        
        # Files that should be skipped (binaries, logs, etc.)
        self.skip_patterns = {
            "*.duckdb", "*.log", "*.tar.gz", "target/", "node_modules/", 
            ".git/", "__pycache__/", "*.pyc", "*.so", "*.dylib", "*.exe",
            "migration_backup/", "archive/", "archive_ap_legacy/"
        }
        
        # File extensions to process
        self.process_extensions = {
            ".rs", ".py", ".js", ".ts", ".tsx", ".json", ".yaml", ".yml", 
            ".toml", ".sql", ".md", ".html", ".css", ".sh", ".env"
        }

    def should_skip_path(self, path: Path) -> bool:
        """Check if a path should be skipped based on patterns."""
        path_str = str(path)
        for pattern in self.skip_patterns:
            if pattern.endswith("/"):
                if pattern[:-1] in path.parts:
                    return True
            elif pattern.startswith("*."):
                if path_str.endswith(pattern[1:]):
                    return True
            elif pattern in path_str:
                return True
        return False

    def should_process_file(self, file_path: Path) -> bool:
        """Check if a file should be processed."""
        if self.should_skip_path(file_path):
            return False
        return file_path.suffix in self.process_extensions

    def create_backup(self) -> None:
        """Create backup of the entire codebase before migration."""
        if self.dry_run:
            logger.info("DRY RUN: Would create backup directory")
            return
            
        if self.backup_dir.exists():
            shutil.rmtree(self.backup_dir)
        
        self.backup_dir.mkdir(parents=True)
        logger.info(f"Creating backup in {self.backup_dir}")
        
        # Backup each root directory
        for root in [self.backend_root, self.frontend_root, self.docs_root]:
            if root.exists():
                backup_target = self.backup_dir / root.name
                shutil.copytree(root, backup_target, ignore=shutil.ignore_patterns(*self.skip_patterns))
                logger.info(f"Backed up {root} to {backup_target}")

    def apply_transformations_to_content(self, content: str, file_path: Path) -> Tuple[str, List[str]]:
        """Apply all transformations to file content and return modified content with change log."""
        modified_content = content
        changes_made = []
        
        for old_pattern, new_pattern in self.transformations:
            # Use word boundaries for most replacements to avoid partial matches
            if old_pattern.islower() and len(old_pattern) > 3:
                # For lowercase words, use word boundaries
                pattern = r'\b' + re.escape(old_pattern) + r'\b'
                new_content = re.sub(pattern, new_pattern, modified_content)
            else:
                # For specific identifiers, constants, etc., use exact match
                new_content = modified_content.replace(old_pattern, new_pattern)
            
            if new_content != modified_content:
                count = modified_content.count(old_pattern) if old_pattern in modified_content else 0
                if count > 0:
                    changes_made.append(f"  {old_pattern} → {new_pattern} ({count} times)")
                modified_content = new_content
        
        return modified_content, changes_made

    def process_file(self, file_path: Path) -> None:
        """Process a single file for symbol → instrument migration."""
        try:
            with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                original_content = f.read()
        except Exception as e:
            logger.warning(f"Could not read {file_path}: {e}")
            return

        modified_content, changes_made = self.apply_transformations_to_content(original_content, file_path)
        
        if modified_content != original_content and changes_made:
            relative_path = file_path.relative_to(Path("/Users/daws/alphapulse"))
            logger.info(f"Processing: {relative_path}")
            for change in changes_made:
                logger.info(change)
            
            self.changes.append({
                "file": str(relative_path),
                "changes": changes_made,
                "lines_modified": len([line for line in changes_made])
            })
            
            if not self.dry_run:
                with open(file_path, 'w', encoding='utf-8') as f:
                    f.write(modified_content)
                logger.info(f"✓ Updated {relative_path}")

    def process_database_schema(self) -> None:
        """Process database schema files for migration."""
        schema_files = [
            self.backend_root / "schema" / "timescaledb_schema.sql",
            # Add other schema files as needed
        ]
        
        for schema_file in schema_files:
            if schema_file.exists():
                logger.info(f"Processing database schema: {schema_file}")
                self.process_file(schema_file)

    def process_directory(self, directory: Path) -> None:
        """Recursively process all files in a directory."""
        if not directory.exists():
            logger.warning(f"Directory does not exist: {directory}")
            return
            
        logger.info(f"Processing directory: {directory}")
        
        for file_path in directory.rglob("*"):
            if file_path.is_file() and self.should_process_file(file_path):
                self.process_file(file_path)

    def update_cargo_toml_references(self) -> None:
        """Update any workspace or crate name references if needed."""
        cargo_files = list(self.backend_root.rglob("Cargo.toml"))
        for cargo_file in cargo_files:
            logger.info(f"Checking Cargo.toml: {cargo_file}")
            self.process_file(cargo_file)

    def generate_migration_report(self) -> None:
        """Generate a detailed report of all changes made."""
        report_path = Path("/Users/daws/alphapulse/migration_report.json")
        
        report = {
            "migration_type": "symbol_to_instrument",
            "dry_run": self.dry_run,
            "total_files_modified": len(self.changes),
            "total_transformations": len(self.transformations),
            "changes": self.changes,
            "summary": {
                "rust_files": len([c for c in self.changes if c["file"].endswith(".rs")]),
                "python_files": len([c for c in self.changes if c["file"].endswith(".py")]),
                "js_ts_files": len([c for c in self.changes if c["file"].endswith((".js", ".ts", ".tsx"))]),
                "config_files": len([c for c in self.changes if c["file"].endswith((".yaml", ".yml", ".toml"))]),
                "doc_files": len([c for c in self.changes if c["file"].endswith(".md")]),
                "sql_files": len([c for c in self.changes if c["file"].endswith(".sql")]),
            }
        }
        
        with open(report_path, 'w') as f:
            json.dump(report, f, indent=2)
        
        logger.info(f"Migration report saved to: {report_path}")
        
        # Print summary
        print("\n" + "="*60)
        print("MIGRATION SUMMARY")
        print("="*60)
        print(f"Mode: {'DRY RUN' if self.dry_run else 'EXECUTION'}")
        print(f"Total files modified: {len(self.changes)}")
        print(f"Rust files: {report['summary']['rust_files']}")
        print(f"Python files: {report['summary']['python_files']}")
        print(f"JS/TS files: {report['summary']['js_ts_files']}")
        print(f"Config files: {report['summary']['config_files']}")
        print(f"Documentation: {report['summary']['doc_files']}")
        print(f"SQL files: {report['summary']['sql_files']}")
        print("="*60)

    def run_migration(self) -> None:
        """Execute the complete migration process."""
        logger.info("Starting Symbol → Instrument migration")
        logger.info(f"Mode: {'DRY RUN' if self.dry_run else 'EXECUTION'}")
        
        # Create backup
        if not self.dry_run:
            self.create_backup()
        
        # Process all directories
        self.process_directory(self.backend_root)
        self.process_directory(self.frontend_root)
        self.process_directory(self.docs_root)
        
        # Process specific files
        self.process_database_schema()
        self.update_cargo_toml_references()
        
        # Generate report
        self.generate_migration_report()
        
        if self.dry_run:
            print("\n🔍 DRY RUN COMPLETE - No files were modified")
            print("Review the migration report and run with --execute to apply changes")
        else:
            print("\n✅ MIGRATION COMPLETE")
            print(f"Backup created at: {self.backup_dir}")
            print("Review the migration report for details")

def main():
    parser = argparse.ArgumentParser(description="Migrate symbol terminology to instrument throughout codebase")
    parser.add_argument("--dry-run", action="store_true", help="Preview changes without applying them")
    parser.add_argument("--execute", action="store_true", help="Apply the migration changes")
    
    args = parser.parse_args()
    
    if not args.dry_run and not args.execute:
        print("Error: Must specify either --dry-run or --execute")
        parser.print_help()
        return
    
    if args.dry_run and args.execute:
        print("Error: Cannot specify both --dry-run and --execute")
        parser.print_help()
        return
    
    migrator = SymbolToInstrumentMigrator(dry_run=args.dry_run)
    migrator.run_migration()

if __name__ == "__main__":
    main()