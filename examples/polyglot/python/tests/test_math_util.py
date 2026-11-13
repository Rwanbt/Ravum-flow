"""Unit tests for the dependency-free Python fixture.

Run from the repository root with `python -m unittest discover -v`.
"""

import unittest

from src.math_util import add


class AddTests(unittest.TestCase):
    def test_adds_positive_numbers(self):
        self.assertEqual(add(2, 3), 5)

    def test_adds_negative_numbers(self):
        self.assertEqual(add(-2, 3), 1)


if __name__ == "__main__":
    unittest.main()
