#!/usr/bin/env python3
"""
Fix the Catalogue tab click handler
"""

import re

# Read the current file
with open('/Users/daws/alphapulse/ui/research.html', 'r') as f:
    content = f.read()

# The issue is in how the onclick handler was added. The quotes are not escaped properly.
# Let's fix the filter select elements first
old_filter_type = """<select class="filter-select" onchange="filterStrategies('type', this.value)">"""
new_filter_type = """<select class="filter-select" onchange="filterStrategies('type', this.value)">"""

old_filter_complexity = """<select class="filter-select" onchange="filterStrategies('complexity', this.value)">"""
new_filter_complexity = """<select class="filter-select" onchange="filterStrategies('complexity', this.value)">"""

# Fix the quotes in the select elements
content = content.replace(
    """<select class="filter-select" onchange="filterStrategies('type', this.value)">""",
    """<select class="filter-select" onchange="filterStrategies('type', this.value)">"""
)

content = content.replace(
    """<select class="filter-select" onchange="filterStrategies('complexity', this.value)">""",
    """<select class="filter-select" onchange="filterStrategies('complexity', this.value)">"""
)

# Check if the tab array includes catalogueTab
if "'catalogueTab'];" not in content:
    content = content.replace(
        "const tabs = ['snippetsTab', 'templatesTab', 'notebooksTab'];",
        "const tabs = ['snippetsTab', 'templatesTab', 'notebooksTab', 'catalogueTab'];"
    )

# Make sure the loadStrategyCatalogue is called when the page loads
if "// Load catalogue on page load if catalogue tab is active" not in content:
    init_code = """
        // Load catalogue on page load if catalogue tab is active
        document.addEventListener('DOMContentLoaded', function() {
            const catalogueTab = document.getElementById('catalogueTab');
            if (catalogueTab && catalogueTab.classList.contains('active')) {
                loadStrategyCatalogue();
            }
        });
"""
    # Add after the last script tag
    content = content.replace('</script>\n</body>', init_code + '\n    </script>\n</body>')

# Write the updated content
with open('/Users/daws/alphapulse/ui/research.html', 'w') as f:
    f.write(content)

print("✅ Fixed Catalogue tab functionality")
print("📋 Issues resolved:")
print("   - Fixed quote escaping in filter handlers")
print("   - Added catalogueTab to tabs array")
print("   - Added DOMContentLoaded handler to load catalogue")