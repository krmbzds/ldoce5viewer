"""Tests for ldoce5viewer.incremental (Maker / Searcher)."""


import pytest

from ldoce5viewer.incremental import IndexError as IncrementalIndexError
from ldoce5viewer.incremental import Maker, Searcher


def _build_index(tmp_path, items):
    """Build an incremental index from *items* and return the index path.

    Each item is a tuple: (plain, typecode, label, path, prio).
    """
    index_path = str(tmp_path / "inc.dat")
    tmp_path2 = str(tmp_path / "inc.tmp")
    maker = Maker(index_path, tmp_path2)
    for plain, typecode, label, p, prio in items:
        maker.add_item(plain, typecode, label, p, prio)
    maker.finalize()
    return index_path


ITEMS = [
    ("apple", "hw", "Apple", "entry/apple", 0),
    ("application", "hw", "Application", "entry/application", 0),
    ("apply", "hw", "Apply", "entry/apply", 1),
    ("banana", "hw", "Banana", "entry/banana", 0),
    ("band", "hw", "Band", "entry/band", 0),
]


class TestIncrementalRoundTrip:
    def test_exact_prefix_match(self, tmp_path):
        index_path = _build_index(tmp_path, ITEMS)
        with Searcher(index_path) as searcher:
            results = searcher.search("appl", limit=10)
        labels = [r[0] for r in results]
        assert "Apple" in labels
        assert "Application" in labels
        assert "Apply" in labels
        # "banana" should not appear
        assert "Banana" not in labels

    def test_full_word_match(self, tmp_path):
        index_path = _build_index(tmp_path, ITEMS)
        with Searcher(index_path) as searcher:
            results = searcher.search("banana", limit=10)
        assert len(results) >= 1
        assert results[0][0] == "Banana"

    def test_no_match_returns_empty(self, tmp_path):
        index_path = _build_index(tmp_path, ITEMS)
        with Searcher(index_path) as searcher:
            results = searcher.search("zzz", limit=10)
        assert results == []

    def test_limit_is_respected(self, tmp_path):
        index_path = _build_index(tmp_path, ITEMS)
        with Searcher(index_path) as searcher:
            results = searcher.search("a", limit=2)
        assert len(results) <= 2

    def test_empty_key_returns_empty(self, tmp_path):
        index_path = _build_index(tmp_path, ITEMS)
        with Searcher(index_path) as searcher:
            results = searcher.search("", limit=10)
        assert results == []

    def test_broken_file_raises(self, tmp_path):
        bad_path = str(tmp_path / "bad.dat")
        (tmp_path / "bad.dat").write_bytes(b"\x00" * 4)
        with pytest.raises(IncrementalIndexError):
            Searcher(bad_path)

    def test_result_structure(self, tmp_path):
        """Each result is (label, path, plain, prio, None)."""
        index_path = _build_index(tmp_path, ITEMS)
        with Searcher(index_path) as searcher:
            results = searcher.search("band", limit=5)
        assert len(results) >= 1
        label, path, plain, prio, extra = results[0]
        assert isinstance(label, str)
        assert isinstance(path, str)
        assert isinstance(plain, str)
        assert isinstance(prio, int)
        assert extra is None
