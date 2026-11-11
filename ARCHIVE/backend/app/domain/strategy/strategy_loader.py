import yaml
from pathlib import Path
from typing import List, Dict, Any
from .strategy_schema import StrategyConfig

def load_strategy_from_yaml(path: str) -> StrategyConfig:
    """Load strategy config từ YAML file."""
    with open(path, 'r', encoding='utf-8') as f:
        data = yaml.safe_load(f)
    return StrategyConfig(**data)

def load_strategy_from_dict(data: Dict[str, Any]) -> StrategyConfig:
    """Load strategy config từ dict (API input)."""
    return StrategyConfig(**data)

def list_available_strategies(directory: str = None) -> List[Dict[str, Any]]:
    """Liệt kê strategies có sẵn trong directory."""
    if directory is None:
        directory = str(Path(__file__).parent / "examples")
    
    strategies = []
    for f in Path(directory).glob("*.yaml"):
        try:
            config = load_strategy_from_yaml(str(f))
            strategies.append({
                "filename": f.name,
                "name": config.name,
                "description": config.description,
                "config": config.model_dump(),
            })
        except Exception:
            pass  # Skip invalid files
    
    return strategies
