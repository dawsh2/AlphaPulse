#!/usr/bin/env python3
"""
Query TLV information from Protocol V2 rustdoc JSON
Shows full struct names with documentation and fields
"""

import json
import sys
from pathlib import Path

def load_rustdoc_json(path="target/doc/protocol_v2.json"):
    """Load the rustdoc JSON file"""
    try:
        with open(path, 'r') as f:
            return json.load(f)
    except FileNotFoundError:
        print(f"Error: {path} not found. Run 'cargo +nightly rustdoc --lib -- --output-format json -Z unstable-options' first")
        sys.exit(1)

def get_tlv_structs(doc):
    """Find all TLV struct types"""
    tlv_structs = []
    
    for id, item in doc.get('index', {}).items():
        name = item.get('name')
        if name and 'TLV' in name and 'inner' in item:
            inner = item['inner']
            if 'struct' in inner:
                tlv_structs.append({
                    'id': id,
                    'name': name,
                    'docs': item.get('docs', 'No documentation'),
                    'fields': inner['struct'].get('kind', {}).get('plain', {}).get('fields', [])
                })
    
    return sorted(tlv_structs, key=lambda x: x['name'])

def get_field_info(doc, field_id):
    """Get field information from ID"""
    field = doc['index'].get(str(field_id), {})
    if 'struct_field' in field.get('inner', {}):
        name = field.get('name', 'unknown')
        # Try to get type information
        type_info = field['inner']['struct_field'].get('type', {})
        type_str = extract_type_string(doc, type_info)
        return f"{name}: {type_str}"
    return None

def extract_type_string(doc, type_info):
    """Extract readable type string from type info"""
    if isinstance(type_info, dict):
        if 'primitive' in type_info:
            return type_info['primitive']
        elif 'array' in type_info:
            elem_type = extract_type_string(doc, type_info['array']['type'])
            length = type_info['array']['len']
            return f"[{elem_type}; {length}]"
        elif 'resolved_path' in type_info:
            path = type_info['resolved_path']
            if 'name' in path:
                return path['name']
            elif 'id' in path:
                # Look up the type name
                type_item = doc['index'].get(str(path['id']), {})
                return type_item.get('name', 'unknown')
    return str(type_info)

def get_tlv_enum_variants(doc):
    """Get TLVType enum variants"""
    variants = []
    
    for id, item in doc.get('index', {}).items():
        if item.get('name') == 'TLVType' and 'enum' in item.get('inner', {}):
            variant_ids = item['inner']['enum'].get('variants', [])
            for var_id in variant_ids:
                var_item = doc['index'].get(str(var_id), {})
                if 'variant' in var_item.get('inner', {}):
                    name = var_item.get('name', 'unknown')
                    docs = var_item.get('docs', '')
                    discriminant = var_item['inner']['variant'].get('discriminant', {})
                    value = discriminant.get('value') if discriminant else None
                    variants.append({
                        'name': name,
                        'value': value,
                        'docs': docs
                    })
    
    return sorted(variants, key=lambda x: x['value'] if x['value'] else 999)

def main():
    doc = load_rustdoc_json()
    
    print("=" * 60)
    print("Protocol V2 TLV Types".center(60))
    print("=" * 60)
    print()
    
    # Show TLV structs
    tlv_structs = get_tlv_structs(doc)
    
    for tlv in tlv_structs:
        print(f"📦 {tlv['name']}")
        
        # Show documentation
        if tlv['docs'] and tlv['docs'] != 'No documentation':
            # Just show first line of docs
            doc_lines = tlv['docs'].split('\n')
            print(f"   {doc_lines[0]}")
        
        # Show fields
        if tlv['fields']:
            print("   Fields:")
            for field_id in tlv['fields']:
                field_info = get_field_info(doc, field_id)
                if field_info:
                    print(f"     • {field_info}")
        
        print()
    
    # Show TLVType enum
    print("=" * 60)
    print("TLV Type Registry (TLVType enum)".center(60))
    print("=" * 60)
    print()
    
    variants = get_tlv_enum_variants(doc)
    
    # Group by domain
    domains = {
        'Market Data': [],
        'Signals': [],
        'Execution': [],
        'Control': [],
        'System': [],
    }
    
    for v in variants:
        if v['value']:
            try:
                val = int(v['value'])
                entry = (val, v['name'], v['docs'])
                if 1 <= val <= 19:
                    domains['Market Data'].append(entry)
                elif 20 <= val <= 39:
                    domains['Signals'].append(entry)
                elif 40 <= val <= 79:
                    domains['Execution'].append(entry)
                elif 80 <= val <= 99:
                    domains['Control'].append(entry)
                elif 100 <= val <= 119:
                    domains['System'].append(entry)
            except (ValueError, TypeError):
                pass
    
    for domain, items in domains.items():
        if items:
            print(f"{domain}:")
            for value, name, docs in sorted(items):
                # Extract first meaningful line from docs
                doc_line = ""
                if docs:
                    lines = [l.strip() for l in docs.split('\n') if l.strip() and not l.strip().startswith('*')]
                    if lines:
                        doc_line = lines[0][:60]  # Truncate long descriptions
                
                print(f"  {value:3} • {name:20} {doc_line}")
            print()

if __name__ == "__main__":
    main()