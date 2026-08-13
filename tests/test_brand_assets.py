from pathlib import Path
import math
import unittest
import xml.etree.ElementTree as ET


REPO_ROOT = Path(__file__).resolve().parents[1]
ASSETS = REPO_ROOT / "assets"
SVG_NS = "{http://www.w3.org/2000/svg}"
MARKS = {
    "invokrum-mark.svg": "0 0 512 512",
    "invokrum-mark-light.svg": "0 0 512 512",
    "invokrum-mark-dark.svg": "0 0 512 512",
    "invokrum-favicon.svg": "0 0 64 64",
    "invokrum-social-square.svg": "0 0 1200 1200",
}
GRAPHICAL_TAGS = {
    f"{SVG_NS}circle",
    f"{SVG_NS}ellipse",
    f"{SVG_NS}line",
    f"{SVG_NS}path",
    f"{SVG_NS}polygon",
    f"{SVG_NS}polyline",
    f"{SVG_NS}rect",
}


def _linear_channel(value: int) -> float:
    channel = value / 255.0
    if channel <= 0.04045:
        return channel / 12.92
    return math.pow((channel + 0.055) / 1.055, 2.4)


def _luminance(color: str) -> float:
    value = color.removeprefix("#")
    red, green, blue = (int(value[index : index + 2], 16) for index in (0, 2, 4))
    return (
        0.2126 * _linear_channel(red)
        + 0.7152 * _linear_channel(green)
        + 0.0722 * _linear_channel(blue)
    )


def _contrast(left: str, right: str) -> float:
    high, low = sorted((_luminance(left), _luminance(right)), reverse=True)
    return (high + 0.05) / (low + 0.05)


class BrandAssetTests(unittest.TestCase):
    def test_svg_assets_are_self_contained_and_accessible(self) -> None:
        for filename, view_box in MARKS.items():
            path = ASSETS / filename
            root = ET.parse(path).getroot()
            self.assertEqual(root.tag, f"{SVG_NS}svg", filename)
            self.assertEqual(root.attrib.get("viewBox"), view_box, filename)
            self.assertEqual(root.attrib.get("role"), "img", filename)
            self.assertEqual(root.attrib.get("aria-labelledby"), "title desc", filename)

            titles = root.findall(f"{SVG_NS}title")
            descriptions = root.findall(f"{SVG_NS}desc")
            self.assertEqual(len(titles), 1, filename)
            self.assertEqual(len(descriptions), 1, filename)
            self.assertTrue((titles[0].text or "").strip(), filename)
            self.assertTrue((descriptions[0].text or "").strip(), filename)

            for element in root.iter():
                local_name = element.tag.removeprefix(SVG_NS)
                self.assertNotEqual(local_name, "text", filename)
                self.assertNotEqual(local_name, "image", filename)
                self.assertNotIn("font-family", element.attrib, filename)
                style = element.attrib.get("style", "").lower()
                self.assertNotIn("font-family", style, filename)
                for attribute, value in element.attrib.items():
                    if attribute.lower().endswith("href"):
                        self.assertFalse(value, filename)

    def test_deterministic_core_is_painted_above_protective_layers(self) -> None:
        for filename in MARKS:
            root = ET.parse(ASSETS / filename).getroot()
            graphical_elements = [element for element in root.iter() if element.tag in GRAPHICAL_TAGS]
            self.assertTrue(graphical_elements, filename)
            self.assertEqual(
                graphical_elements[-1].tag,
                f"{SVG_NS}circle",
                f"{filename} should paint the blue core after the protective layers",
            )

    def test_favicon_uses_simplified_geometry(self) -> None:
        root = ET.parse(ASSETS / "invokrum-favicon.svg").getroot()
        graphical_elements = [element for element in root.iter() if element.tag in GRAPHICAL_TAGS]
        self.assertLessEqual(
            len(graphical_elements),
            5,
            "favicon geometry should stay intentionally simple for 16px and 32px rendering",
        )

    def test_representative_brand_contrast_pairs_remain_distinct(self) -> None:
        pairs = [
            ("#17232c", "#ffffff"),
            ("#237eaf", "#17232c"),
            ("#328a58", "#17232c"),
            ("#88a5b7", "#0b1218"),
            ("#58c8e8", "#243440"),
            ("#72cf79", "#243440"),
        ]
        for foreground, background in pairs:
            self.assertGreaterEqual(
                _contrast(foreground, background),
                3.0,
                f"{foreground} should remain visually distinct from {background}",
            )


if __name__ == "__main__":
    unittest.main()
