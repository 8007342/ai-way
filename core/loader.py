"""
Agent Profile Loader

Parses agent markdown files from the agents repository into structured data
that can be used to generate Ollama modelfiles.
"""

import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class AgentProfile:
    """Structured representation of an agent profile."""

    name: str
    category: str
    filepath: Path

    role: str = ""
    expertise: list[str] = field(default_factory=list)
    personality_traits: list[str] = field(default_factory=list)
    responsibilities: list[str] = field(default_factory=list)
    working_style: list[str] = field(default_factory=list)
    use_cases: list[str] = field(default_factory=list)

    # Optional sections some agents have
    collaboration_style: list[str] = field(default_factory=list)
    red_flags: list[str] = field(default_factory=list)
    philosophy: str = ""

    # Raw content for special cases
    raw_content: str = ""


def parse_agent_profile(filepath: Path) -> AgentProfile:
    """
    Parse a single agent markdown file into an AgentProfile.

    Args:
        filepath: Path to the agent markdown file

    Returns:
        AgentProfile with extracted sections
    """
    content = filepath.read_text(encoding="utf-8")

    # Extract agent name from filename
    name = filepath.stem

    # Extract category from parent directory
    category = filepath.parent.name

    profile = AgentProfile(
        name=name,
        category=category,
        filepath=filepath,
        raw_content=content,
    )

    # Parse sections using regex
    sections = _extract_sections(content)

    # Map sections to profile fields
    if "Role" in sections:
        profile.role = sections["Role"].strip()

    if "Expertise" in sections:
        profile.expertise = _parse_bullet_list(sections["Expertise"])

    if "Personality Traits" in sections:
        profile.personality_traits = _parse_bullet_list(sections["Personality Traits"])

    if "Primary Responsibilities" in sections:
        profile.responsibilities = _parse_bullet_list(sections["Primary Responsibilities"])

    if "Working Style" in sections:
        profile.working_style = _parse_bullet_list(sections["Working Style"])

    if "Use Cases" in sections:
        profile.use_cases = _parse_bullet_list(sections["Use Cases"])

    if "Collaboration Style" in sections:
        profile.collaboration_style = _parse_bullet_list(sections["Collaboration Style"])

    if "Red Flags" in sections:
        profile.red_flags = _parse_bullet_list(sections["Red Flags"])

    # Look for philosophy/mantra (often in blockquotes at the end)
    philosophy_match = re.search(r'>\s*["\']?([^"\'\n]+)["\']?\s*$', content, re.MULTILINE)
    if philosophy_match:
        profile.philosophy = philosophy_match.group(1).strip()

    return profile


def _extract_sections(content: str) -> dict[str, str]:
    """
    Extract H2 sections from markdown content.

    Returns:
        Dict mapping section names to their content
    """
    sections = {}

    # Split by H2 headers (## Section Name)
    pattern = r'^##\s+(.+?)$'
    matches = list(re.finditer(pattern, content, re.MULTILINE))

    for i, match in enumerate(matches):
        section_name = match.group(1).strip()
        start = match.end()

        # End is either next section or end of content
        if i + 1 < len(matches):
            end = matches[i + 1].start()
        else:
            end = len(content)

        sections[section_name] = content[start:end].strip()

    return sections


def _parse_bullet_list(text: str) -> list[str]:
    """
    Parse a bullet list from markdown text.

    Handles:
    - Simple bullets (- item)
    - Bold categories (- **Category**: description)
    - Nested lists (with indentation)
    """
    items = []

    # Match lines starting with - or *
    pattern = r'^[-*]\s+(.+)$'

    for match in re.finditer(pattern, text, re.MULTILINE):
        item = match.group(1).strip()

        # Clean up bold markers but preserve content
        item = re.sub(r'\*\*([^*]+)\*\*', r'\1', item)

        if item:
            items.append(item)

    return items


def load_all_agents(agents_path: Path) -> list[AgentProfile]:
    """
    Load all agent profiles from the agents repository.

    Args:
        agents_path: Path to the agents repository root

    Returns:
        List of all parsed AgentProfiles
    """
    agents = []

    # Categories to scan (directories containing agent .md files)
    categories = [
        "developers",
        "architects",
        "design",
        "data-specialists",
        "domain-experts",
        "security",
        "legal",
        "qa",
        "research",
        "specialists",
    ]

    for category in categories:
        category_path = agents_path / category
        if not category_path.exists():
            continue

        for md_file in category_path.glob("*.md"):
            try:
                profile = parse_agent_profile(md_file)
                agents.append(profile)
            except Exception as e:
                print(f"Warning: Failed to parse {md_file}: {e}")

    return agents


def load_constitution(agents_path: Path) -> str:
    """
    Load the Constitution (Five Laws of Evolution) from the agents repository.

    Returns abbreviated version suitable for embedding in modelfiles.
    """
    constitution_path = agents_path / "CONSTITUTION.md"

    if not constitution_path.exists():
        return ""

    content = constitution_path.read_text(encoding="utf-8")

    # Extract the Five Laws section
    laws_match = re.search(
        r'## The Five Laws of Evolution.*?```(.*?)```',
        content,
        re.DOTALL
    )

    if laws_match:
        laws_block = laws_match.group(1).strip()
        # Clean up the ASCII box drawing
        lines = []
        for line in laws_block.split('\n'):
            # Skip box borders
            if re.match(r'^[┌├└│─┐┤┘]+$', line.strip()):
                continue
            # Clean up remaining box characters
            line = re.sub(r'[│┌┐└┘├┤─]', '', line)
            line = line.strip()
            if line:
                lines.append(line)
        return '\n'.join(lines)

    return ""


if __name__ == "__main__":
    # Quick test
    import sys

    if len(sys.argv) > 1:
        agents_path = Path(sys.argv[1])
    else:
        # Default to sibling agents directory
        agents_path = Path(__file__).parent.parent.parent / "agents"

    print(f"Loading agents from: {agents_path}")

    agents = load_all_agents(agents_path)
    print(f"\nLoaded {len(agents)} agents:\n")

    for agent in agents:
        print(f"  [{agent.category}] {agent.name}")
        print(f"    Role: {agent.role[:60]}..." if len(agent.role) > 60 else f"    Role: {agent.role}")
        print(f"    Expertise items: {len(agent.expertise)}")
        print(f"    Personality traits: {len(agent.personality_traits)}")
        print()

    print("\nConstitution (abbreviated):")
    print(load_constitution(agents_path))
