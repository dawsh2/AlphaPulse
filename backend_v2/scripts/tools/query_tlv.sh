#!/bin/bash

# Query TLV types from rustdoc JSON with full information

echo "=== Protocol V2 TLV Types ==="
echo ""

# Function to get field info for a struct
get_field_info() {
    local struct_id=$1
    local json_file=$2
    
    jq -r --arg id "$struct_id" '
        .index[$id] | 
        if .inner.struct_field then
            "\(.name): \(.inner.struct_field.type | tostring)"
        else
            ""
        end
    ' "$json_file" 2>/dev/null
}

# Find all TLV structs with documentation
jq -r '
    .index | to_entries[] | 
    select(.value.name) | 
    select(.value.name | endswith("TLV")) |
    select(.value.inner.struct) |
    {
        name: .value.name,
        docs: .value.docs,
        id: .key
    }
' target/doc/protocol_v2.json 2>/dev/null | while IFS= read -r line; do
    # Parse JSON line
    name=$(echo "$line" | jq -r '.name' 2>/dev/null)
    docs=$(echo "$line" | jq -r '.docs // "No documentation"' 2>/dev/null)
    id=$(echo "$line" | jq -r '.id' 2>/dev/null)
    
    if [ ! -z "$name" ]; then
        echo "📦 $name"
        echo "   $docs"
        
        # Get field IDs for this struct
        field_ids=$(jq -r --arg id "$id" '
            .index[$id].inner.struct.kind.plain.fields[]? // empty
        ' target/doc/protocol_v2.json 2>/dev/null)
        
        if [ ! -z "$field_ids" ]; then
            echo "   Fields:"
            for field_id in $field_ids; do
                field_info=$(get_field_info "$field_id" "target/doc/protocol_v2.json")
                if [ ! -z "$field_info" ]; then
                    echo "     • $field_info"
                fi
            done
        fi
        echo ""
    fi
done

# Also show TLVType enum variants
echo "=== TLV Type Registry ==="
echo ""

jq -r '
    .index | to_entries[] | 
    select(.value.name == "TLVType") |
    .value.inner.enum.variants[]? // empty
' target/doc/protocol_v2.json 2>/dev/null | while IFS= read -r variant_id; do
    if [ ! -z "$variant_id" ]; then
        variant_info=$(jq -r --arg id "$variant_id" '
            .index[$id] | 
            if .name then
                if .inner.variant.discriminant then
                    "\(.inner.variant.discriminant.value) - \(.name): \(.docs // "")"
                else
                    "\(.name): \(.docs // "")"
                end
            else
                ""
            end
        ' target/doc/protocol_v2.json 2>/dev/null)
        
        if [ ! -z "$variant_info" ]; then
            echo "  $variant_info"
        fi
    fi
done