from typing import Dict, List, Optional
from app.services.data_providers.base_provider import (
    MarketDataProviderAdapter,
    ProviderMetadata,
    ProviderException
)
from app.services.data_providers.ssi_provider import SSIProviderAdapter
from app.services.data_providers.vnstock_provider import VnstockProviderAdapter

class ProviderRegistry:
    """Registry factory managing active MarketDataProviderAdapter instances."""

    _instance: Optional['ProviderRegistry'] = None

    def __init__(self):
        self._providers: Dict[str, MarketDataProviderAdapter] = {}
        self._register_default_providers()

    @classmethod
    def get_instance(cls) -> 'ProviderRegistry':
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    def _register_default_providers(self):
        self.register_provider(SSIProviderAdapter())
        self.register_provider(VnstockProviderAdapter())

    def register_provider(self, adapter: MarketDataProviderAdapter):
        meta = adapter.get_metadata()
        self._providers[meta.provider_id] = adapter

    def get_provider(self, provider_id: str) -> Optional[MarketDataProviderAdapter]:
        return self._providers.get(provider_id.lower().strip())

    def list_providers(self) -> List[ProviderMetadata]:
        return [adapter.get_metadata() for adapter in self._providers.values()]
