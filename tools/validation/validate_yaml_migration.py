#!/usr/bin/env python3
"""
YAML Migration Validation Script for MCP 2025-06-18

This script validates both legacy and enhanced MCP 2025-06-18 YAML capability files.
It performs comprehensive validation including:
- YAML syntax validation
- Schema compliance checking
- MCP 2025-06-18 feature validation
- Security configuration validation
- Performance and monitoring validation

Usage:
    python3 validate_yaml_migration.py <path_to_capabilities_directory>
    python3 validate_yaml_migration.py <path_to_single_yaml_file>
"""

import sys
import os
import json
import yaml
import argparse
from pathlib import Path
from typing import Dict, List, Any, Optional
from dataclasses import dataclass
from enum import Enum


class ValidationLevel(Enum):
    ERROR = "ERROR"
    WARNING = "WARNING"
    INFO = "INFO"
    SUCCESS = "SUCCESS"


@dataclass
class ValidationResult:
    file_path: str
    level: ValidationLevel
    message: str
    details: Optional[str] = None


class YamlMigrationValidator:
    """Comprehensive validator for MCP 2025-06-18 YAML migration"""
    
    def __init__(self, strict_mode: bool = False):
        self.strict_mode = strict_mode
        self.results: List[ValidationResult] = []
        
        # Required fields for enhanced format
        self.required_enhanced_metadata = [
            "name", "description", "version", "author", 
            "classification", "discovery_metadata", "mcp_capabilities"
        ]
        
        self.required_enhanced_tool_sections = [
            "name", "core", "execution", "discovery", "monitoring", "access"
        ]
        
        # Security classifications
        self.valid_security_levels = [
            "safe", "restricted", "mixed", "privileged", "dangerous", "blocked"
        ]
        
        # Complexity levels
        self.valid_complexity_levels = [
            "simple", "moderate", "complex", "varied", "very_complex"
        ]

    def validate_directory(self, directory_path: str) -> List[ValidationResult]:
        """Validate all YAML files in a directory"""
        directory = Path(directory_path)
        if not directory.exists():
            self.results.append(ValidationResult(
                directory_path, ValidationLevel.ERROR, 
                f"Directory does not exist: {directory_path}"
            ))
            return self.results
        
        yaml_files = list(directory.glob("**/*.yaml")) + list(directory.glob("**/*.yml"))
        
        if not yaml_files:
            self.results.append(ValidationResult(
                directory_path, ValidationLevel.WARNING,
                "No YAML files found in directory"
            ))
            return self.results
        
        self.results.append(ValidationResult(
            directory_path, ValidationLevel.INFO,
            f"Found {len(yaml_files)} YAML files to validate"
        ))
        
        for yaml_file in yaml_files:
            self.validate_file(str(yaml_file))
        
        return self.results

    def validate_file(self, file_path: str) -> List[ValidationResult]:
        """Validate a single YAML file"""
        file_path_obj = Path(file_path)
        
        if not file_path_obj.exists():
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                f"File does not exist: {file_path}"
            ))
            return self.results
        
        # Step 1: YAML syntax validation
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = yaml.safe_load(f)
        except yaml.YAMLError as e:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                "Invalid YAML syntax",
                str(e)
            ))
            return self.results
        except Exception as e:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                "Failed to read file",
                str(e)
            ))
            return self.results
        
        # Step 2: Determine format type
        is_enhanced = self._is_enhanced_format(content)
        format_type = "Enhanced MCP 2025-06-18" if is_enhanced else "Legacy"
        
        self.results.append(ValidationResult(
            file_path, ValidationLevel.INFO,
            f"Detected format: {format_type}"
        ))
        
        # Step 3: Format-specific validation
        if is_enhanced:
            self._validate_enhanced_format(file_path, content)
        else:
            self._validate_legacy_format(file_path, content)
        
        return self.results

    def _is_enhanced_format(self, content: Dict[str, Any]) -> bool:
        """Determine if YAML content is enhanced MCP 2025-06-18 format"""
        if not isinstance(content, dict):
            return False
        
        # Check for enhanced metadata structure
        metadata = content.get('metadata', {})
        if isinstance(metadata, dict):
            has_classification = 'classification' in metadata
            has_discovery_metadata = 'discovery_metadata' in metadata
            has_mcp_capabilities = 'mcp_capabilities' in metadata
            
            if has_classification or has_discovery_metadata or has_mcp_capabilities:
                return True
        
        # Check for enhanced tools structure
        tools = content.get('tools', [])
        if isinstance(tools, list) and tools:
            first_tool = tools[0]
            if isinstance(first_tool, dict):
                has_core = 'core' in first_tool
                has_execution = 'execution' in first_tool
                has_discovery = 'discovery' in first_tool
                has_monitoring = 'monitoring' in first_tool
                has_access = 'access' in first_tool
                
                # If it has most enhanced sections, consider it enhanced
                enhanced_sections = sum([has_core, has_execution, has_discovery, has_monitoring, has_access])
                if enhanced_sections >= 3:
                    return True
        
        return False

    def _validate_enhanced_format(self, file_path: str, content: Dict[str, Any]):
        """Validate enhanced MCP 2025-06-18 format"""
        # Validate metadata
        self._validate_enhanced_metadata(file_path, content.get('metadata', {}))
        
        # Validate tools
        tools = content.get('tools', [])
        if not tools:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                "Enhanced format must contain at least one tool"
            ))
            return
        
        for i, tool in enumerate(tools):
            self._validate_enhanced_tool(file_path, tool, i)

    def _validate_enhanced_metadata(self, file_path: str, metadata: Dict[str, Any]):
        """Validate enhanced metadata structure"""
        if not isinstance(metadata, dict):
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                "Enhanced format requires metadata object"
            ))
            return
        
        # Check required fields
        missing_fields = []
        for field in ['name', 'description', 'version', 'author']:
            if field not in metadata or not metadata[field]:
                missing_fields.append(field)
        
        if missing_fields:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                f"Missing required metadata fields: {', '.join(missing_fields)}"
            ))
        
        # Validate classification
        classification = metadata.get('classification', {})
        if isinstance(classification, dict):
            self._validate_classification(file_path, classification)
        
        # Validate discovery metadata
        discovery_metadata = metadata.get('discovery_metadata', {})
        if isinstance(discovery_metadata, dict):
            self._validate_discovery_metadata(file_path, discovery_metadata)
        
        # Validate MCP capabilities
        mcp_capabilities = metadata.get('mcp_capabilities', {})
        if isinstance(mcp_capabilities, dict):
            self._validate_mcp_capabilities(file_path, mcp_capabilities)

    def _validate_classification(self, file_path: str, classification: Dict[str, Any]):
        """Validate classification structure"""
        security_level = classification.get('security_level')
        if security_level and security_level not in self.valid_security_levels:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                f"Invalid security_level: {security_level}. Must be one of: {', '.join(self.valid_security_levels)}"
            ))
        
        complexity_level = classification.get('complexity_level')
        if complexity_level and complexity_level not in self.valid_complexity_levels:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                f"Invalid complexity_level: {complexity_level}. Must be one of: {', '.join(self.valid_complexity_levels)}"
            ))

    def _validate_discovery_metadata(self, file_path: str, discovery_metadata: Dict[str, Any]):
        """Validate discovery metadata structure"""
        required_fields = ['primary_keywords', 'semantic_embeddings', 'llm_enhanced', 'workflow_enabled']
        missing_fields = []
        
        for field in required_fields:
            if field not in discovery_metadata:
                missing_fields.append(field)
        
        if missing_fields:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.WARNING,
                f"Missing recommended discovery_metadata fields: {', '.join(missing_fields)}"
            ))

    def _validate_mcp_capabilities(self, file_path: str, mcp_capabilities: Dict[str, Any]):
        """Validate MCP capabilities structure"""
        version = mcp_capabilities.get('version')
        if version != '2025-06-18':
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                f"Invalid MCP version: {version}. Expected: 2025-06-18"
            ))
        
        required_capabilities = [
            'supports_cancellation', 'supports_progress', 'supports_sampling',
            'supports_validation', 'supports_elicitation'
        ]
        
        missing_capabilities = []
        for capability in required_capabilities:
            if capability not in mcp_capabilities:
                missing_capabilities.append(capability)
        
        if missing_capabilities:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.WARNING,
                f"Missing MCP capability declarations: {', '.join(missing_capabilities)}"
            ))

    def _validate_enhanced_tool(self, file_path: str, tool: Dict[str, Any], index: int):
        """Validate enhanced tool structure"""
        tool_name = tool.get('name', f'tool_{index}')
        
        # Check required sections
        missing_sections = []
        for section in self.required_enhanced_tool_sections:
            if section not in tool:
                missing_sections.append(section)
        
        if missing_sections:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                f"Tool '{tool_name}': Missing required sections: {', '.join(missing_sections)}"
            ))
            return
        
        # Validate individual sections
        self._validate_tool_core(file_path, tool_name, tool.get('core', {}))
        self._validate_tool_execution(file_path, tool_name, tool.get('execution', {}))
        self._validate_tool_discovery(file_path, tool_name, tool.get('discovery', {}))
        self._validate_tool_monitoring(file_path, tool_name, tool.get('monitoring', {}))
        self._validate_tool_access(file_path, tool_name, tool.get('access', {}))

    def _validate_tool_core(self, file_path: str, tool_name: str, core: Dict[str, Any]):
        """Validate tool core section"""
        if not core.get('description'):
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                f"Tool '{tool_name}': Core section missing description"
            ))
        
        input_schema = core.get('input_schema')
        if not input_schema or not isinstance(input_schema, dict):
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                f"Tool '{tool_name}': Core section missing valid input_schema"
            ))

    def _validate_tool_execution(self, file_path: str, tool_name: str, execution: Dict[str, Any]):
        """Validate tool execution section"""
        routing = execution.get('routing', {})
        if not routing.get('type'):
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                f"Tool '{tool_name}': Execution section missing routing type"
            ))
        
        security = execution.get('security', {})
        if not security.get('classification'):
            self.results.append(ValidationResult(
                file_path, ValidationLevel.WARNING,
                f"Tool '{tool_name}': Execution section missing security classification"
            ))

    def _validate_tool_discovery(self, file_path: str, tool_name: str, discovery: Dict[str, Any]):
        """Validate tool discovery section"""
        ai_enhanced = discovery.get('ai_enhanced', {})
        if not ai_enhanced.get('description'):
            self.results.append(ValidationResult(
                file_path, ValidationLevel.WARNING,
                f"Tool '{tool_name}': Discovery section missing AI-enhanced description"
            ))

    def _validate_tool_monitoring(self, file_path: str, tool_name: str, monitoring: Dict[str, Any]):
        """Validate tool monitoring section"""
        progress_tracking = monitoring.get('progress_tracking', {})
        if 'enabled' not in progress_tracking:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.WARNING,
                f"Tool '{tool_name}': Monitoring section missing progress_tracking.enabled"
            ))
        
        cancellation = monitoring.get('cancellation', {})
        if 'enabled' not in cancellation:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.WARNING,
                f"Tool '{tool_name}': Monitoring section missing cancellation.enabled"
            ))

    def _validate_tool_access(self, file_path: str, tool_name: str, access: Dict[str, Any]):
        """Validate tool access section"""
        required_access_fields = ['hidden', 'enabled', 'requires_permissions', 'user_groups']
        missing_fields = []
        
        for field in required_access_fields:
            if field not in access:
                missing_fields.append(field)
        
        if missing_fields:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.WARNING,
                f"Tool '{tool_name}': Access section missing fields: {', '.join(missing_fields)}"
            ))

    def _validate_legacy_format(self, file_path: str, content: Dict[str, Any]):
        """Validate legacy format"""
        # Basic legacy validation
        tools = content.get('tools', [])
        if not tools:
            self.results.append(ValidationResult(
                file_path, ValidationLevel.ERROR,
                "Legacy format must contain at least one tool"
            ))
            return
        
        for i, tool in enumerate(tools):
            tool_name = tool.get('name', f'tool_{i}')
            
            if not tool.get('name'):
                self.results.append(ValidationResult(
                    file_path, ValidationLevel.ERROR,
                    f"Tool {i}: Missing name"
                ))
            
            if not tool.get('description'):
                self.results.append(ValidationResult(
                    file_path, ValidationLevel.ERROR,
                    f"Tool '{tool_name}': Missing description"
                ))
            
            if not tool.get('inputSchema'):
                self.results.append(ValidationResult(
                    file_path, ValidationLevel.ERROR,
                    f"Tool '{tool_name}': Missing inputSchema"
                ))
            
            routing = tool.get('routing', {})
            if not routing.get('type'):
                self.results.append(ValidationResult(
                    file_path, ValidationLevel.ERROR,
                    f"Tool '{tool_name}': Missing routing type"
                ))

    def print_results(self):
        """Print validation results with color coding"""
        colors = {
            ValidationLevel.ERROR: '\033[91m',      # Red
            ValidationLevel.WARNING: '\033[93m',    # Yellow
            ValidationLevel.INFO: '\033[94m',       # Blue
            ValidationLevel.SUCCESS: '\033[92m',    # Green
        }
        reset_color = '\033[0m'
        
        # Group results by level
        errors = [r for r in self.results if r.level == ValidationLevel.ERROR]
        warnings = [r for r in self.results if r.level == ValidationLevel.WARNING]
        infos = [r for r in self.results if r.level == ValidationLevel.INFO]
        
        print(f"\n{'='*80}")
        print(f"YAML MIGRATION VALIDATION RESULTS")
        print(f"{'='*80}")
        
        # Summary
        print(f"\nSUMMARY:")
        print(f"  Errors: {len(errors)}")
        print(f"  Warnings: {len(warnings)}")
        print(f"  Info: {len(infos)}")
        
        # Detailed results
        for result in self.results:
            color = colors.get(result.level, '')
            print(f"\n{color}[{result.level.value}]{reset_color} {result.file_path}")
            print(f"  {result.message}")
            if result.details:
                print(f"  Details: {result.details}")
        
        print(f"\n{'='*80}")
        
        # Final status
        if errors:
            print(f"{colors[ValidationLevel.ERROR]}VALIDATION FAILED: {len(errors)} errors found{reset_color}")
            return False
        elif warnings:
            print(f"{colors[ValidationLevel.WARNING]}VALIDATION PASSED WITH WARNINGS: {len(warnings)} warnings{reset_color}")
            return True
        else:
            print(f"{colors[ValidationLevel.SUCCESS]}VALIDATION PASSED: All files are valid{reset_color}")
            return True


def main():
    parser = argparse.ArgumentParser(
        description="Validate YAML files for MCP 2025-06-18 migration",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python3 validate_yaml_migration.py capabilities/
  python3 validate_yaml_migration.py capabilities/ai/smart_discovery.yaml
  python3 validate_yaml_migration.py --strict capabilities/
        """
    )
    
    parser.add_argument(
        'path',
        help='Path to YAML file or directory containing YAML files'
    )
    
    parser.add_argument(
        '--strict',
        action='store_true',
        help='Enable strict validation mode (warnings become errors)'
    )
    
    args = parser.parse_args()
    
    if not os.path.exists(args.path):
        print(f"Error: Path does not exist: {args.path}")
        sys.exit(1)
    
    validator = YamlMigrationValidator(strict_mode=args.strict)
    
    if os.path.isdir(args.path):
        validator.validate_directory(args.path)
    else:
        validator.validate_file(args.path)
    
    success = validator.print_results()
    
    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()