#!/usr/bin/env python3
"""
Add glossary hyperlinks to README files.
Links terms to their definitions in GLOSSARY.md
"""

import os
import re
from pathlib import Path

# Define terms and their glossary anchors
# Format: (term, anchor, case_sensitive)
GLOSSARY_TERMS = [
    # Midnight-Specific (high priority - link these first)
    ("ZSwap", "zswap", True),
    ("cNIGHT", "cnight", True),
    ("DUST", "dust", True),
    ("tDUST", "tdust", True),
    ("Glacier Drop", "glacier-drop", False),
    ("Compact", "compact", True),
    
    # Substrate/Polkadot
    ("AURA", "aura-authority-round", True),
    ("BEEFY", "beefy-bridge-efficiency-enabling-finality-yielder", True),
    ("GRANDPA", "grandpa-ghost-based-recursive-ancestor-deriving-prefix-agreement", True),
    ("FRAME", "frame-framework-for-runtime-aggregation-of-modularized-entities", True),
    ("Pallet", "pallet", True),
    ("Runtime", "runtime", True),
    ("Extrinsic", "extrinsic", True),
    ("Inherent", "inherent", True),
    ("Dispatchable", "dispatchable", True),
    ("Weight", "weight", True),
    
    # Cardano
    ("db-sync", "db-sync", True),
    ("Partner Chain", "partner-chain", False),
    ("Policy ID", "policy-id", False),
    
    # Cryptographic
    ("Halo2", "halo2", True),
    ("SNARK", "snark-succinct-non-interactive-argument-of-knowledge", True),
    ("Zero-Knowledge Proof", "zero-knowledge-proof-zkp", False),
    ("ZKP", "zero-knowledge-proof-zkp", True),
    ("Merkle Tree", "merkle-tree", False),
    ("UTXO", "utxo-unspent-transaction-output", True),
    
    # Governance
    ("Federated Authority", "federated-authority", False),
    ("Technical Committee", "technical-committee", False),
    ("Council", "council", True),
    
    # Transaction & State
    ("CardanoPosition", "cardanoposition", True),
    ("Block Context", "block-context", False),
    ("System Transaction", "system-transaction", False),
    
    # Network
    ("Chain Spec", "chain-spec--chain-specification", False),
    ("Genesis", "genesis", True),
    ("Testnet", "testnet", True),
    ("Mainnet", "mainnet", True),
    ("Devnet", "devnet", True),
    
    # Development
    ("Host Function", "host-function", False),
    ("Runtime API", "runtime-api", False),
    ("Benchmarking", "benchmarking", True),
]

def get_relative_glossary_path(readme_path, root_dir):
    """Calculate relative path from README to GLOSSARY.md"""
    readme_dir = os.path.dirname(readme_path)
    rel_path = os.path.relpath(root_dir, readme_dir)
    return os.path.join(rel_path, "GLOSSARY.md")

def is_in_code_block(content, pos):
    """Check if position is inside a code block"""
    # Count ``` before position
    before = content[:pos]
    fence_count = before.count("```")
    return fence_count % 2 == 1

def is_in_inline_code(content, pos):
    """Check if position is inside inline code (single backticks)"""
    # Look for backticks around position
    before = content[:pos]
    after = content[pos:]
    
    # Find last backtick before pos
    last_backtick = before.rfind("`")
    if last_backtick == -1:
        return False
    
    # Check if it's part of a fence
    if before[max(0, last_backtick-2):last_backtick+1] == "```":
        return False
    
    # Find next backtick after pos
    next_backtick = after.find("`")
    if next_backtick == -1:
        return False
    
    # Check if it's part of a fence
    if after[next_backtick:next_backtick+3] == "```":
        return False
    
    # Count backticks between last_backtick and pos
    between = before[last_backtick+1:]
    if "`" not in between:
        return True
    
    return False

def is_already_linked(content, pos, term_len):
    """Check if the term is already part of a markdown link"""
    # Check if preceded by [ or followed by ]( or ](
    before = content[max(0, pos-50):pos]
    after = content[pos:pos+term_len+50]
    
    # Already in a link text [term]
    if "](" in after[:term_len+5] or "](#" in after[:term_len+5]:
        return True
    if "[" in before[-20:] and "]" not in before[-20:]:
        return True
    
    return False

def is_in_heading(content, pos):
    """Check if position is in a markdown heading"""
    # Find start of line
    line_start = content.rfind("\n", 0, pos) + 1
    line = content[line_start:pos]
    return line.lstrip().startswith("#")

def add_glossary_links(content, glossary_path):
    """Add glossary links to content"""
    result = content
    
    for term, anchor, case_sensitive in GLOSSARY_TERMS:
        # Build pattern
        if case_sensitive:
            pattern = r'\b' + re.escape(term) + r'\b'
            flags = 0
        else:
            pattern = r'\b' + re.escape(term) + r'\b'
            flags = re.IGNORECASE
        
        # Find all matches
        matches = list(re.finditer(pattern, result, flags))
        
        # Process matches in reverse order to preserve positions
        for match in reversed(matches):
            pos = match.start()
            matched_text = match.group()
            
            # Skip if in code block, inline code, already linked, or in heading
            if is_in_code_block(result, pos):
                continue
            if is_in_inline_code(result, pos):
                continue
            if is_already_linked(result, pos, len(matched_text)):
                continue
            if is_in_heading(result, pos):
                continue
            
            # Create link
            link = f"[{matched_text}]({glossary_path}#{anchor})"
            
            # Replace only first occurrence per paragraph to avoid over-linking
            # Find paragraph boundaries
            para_start = result.rfind("\n\n", 0, pos)
            para_end = result.find("\n\n", pos)
            if para_end == -1:
                para_end = len(result)
            
            paragraph = result[para_start:para_end]
            
            # Check if already linked this term in this paragraph
            if f"]({glossary_path}#{anchor})" in paragraph[:pos-para_start]:
                continue
            
            # Replace
            result = result[:pos] + link + result[pos + len(matched_text):]
    
    return result

def process_readme(readme_path, root_dir):
    """Process a single README file"""
    with open(readme_path, 'r') as f:
        content = f.read()
    
    glossary_path = get_relative_glossary_path(readme_path, root_dir)
    
    new_content = add_glossary_links(content, glossary_path)
    
    if new_content != content:
        with open(readme_path, 'w') as f:
            f.write(new_content)
        return True
    return False

def main():
    root_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    
    # Find all README.md files (excluding target, .git, node_modules)
    readme_files = []
    for root, dirs, files in os.walk(root_dir):
        # Skip unwanted directories
        dirs[:] = [d for d in dirs if d not in ['target', '.git', 'node_modules', '.ai']]
        
        for file in files:
            if file == "README.md":
                filepath = os.path.join(root, file)
                # Skip the root README and GLOSSARY
                if filepath == os.path.join(root_dir, "README.md"):
                    continue
                readme_files.append(filepath)
    
    print(f"Found {len(readme_files)} README files to process")
    
    updated = 0
    for readme in sorted(readme_files):
        rel_path = os.path.relpath(readme, root_dir)
        if process_readme(readme, root_dir):
            print(f"  Updated: {rel_path}")
            updated += 1
        else:
            print(f"  No changes: {rel_path}")
    
    print(f"\nUpdated {updated} files")

if __name__ == "__main__":
    main()

