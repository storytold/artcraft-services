"""Regression checks for catalog-export validation failures."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("catalog_sync", Path(__file__).with_name("sync-launcher-catalog.py"))
catalog_sync = importlib.util.module_from_spec(spec)
spec.loader.exec_module(catalog_sync)


class CatalogExportValidation(unittest.TestCase):
    """Exporter format changes must not silently omit icons or attribution."""

    def test_missing_or_ambiguous_icon_anchor_names_app(self):
        """Both zero matches and duplicate anchors fail before writing output."""
        for catalog in ['', 'startup_wm_class = "example"\n' * 2]:
            with self.subTest(catalog=catalog), self.assertRaisesRegex(ValueError, "example-app"):
                catalog_sync.add_icon(catalog, {"id": "example-app", "startup_wm_class": "example"}, "icon.png", "hash")

    def test_icon_metadata_is_inserted(self):
        """A unique exporter anchor receives both required fields."""
        result = catalog_sync.add_icon('startup_wm_class = "example"\n', {"id": "example-app", "startup_wm_class": "example"}, "icon.png", "hash")
        self.assertIn('icon_url = "https://getartcraft.com/launcher-app-icons/icon.png"', result)
        self.assertIn('icon_sha256 = "hash"', result)

    def test_missing_attribution_names_app(self):
        """Missing source attribution produces an actionable error."""
        with self.assertRaisesRegex(ValueError, "Missing attribution row for example"):
            catalog_sync.attribution_row([], "example")

    def test_existing_attribution_is_preserved(self):
        """Matching retains the exact source row."""
        row = '| `assets/app-icons/example.png` | Example |'
        self.assertEqual(catalog_sync.attribution_row([row], "example"), row)


if __name__ == "__main__":
    unittest.main()
