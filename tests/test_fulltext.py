"""Tests for ldoce5viewer.fulltext (Maker / Searcher / VariationsReader)."""

import pytest

try:
    import whoosh  # noqa: F401
except ImportError:
    pytest.skip("whoosh not installed", allow_module_level=True)

from ldoce5viewer.fulltext import Maker, Searcher, VariationsReader, VariationsWriter

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

ITEMS = [
    # (itemtype, content, asfilter, label, path, prio, sortkey)
    ("hw", "run", "hw", "Run", "entry/run", 0, "run"),
    ("hw", "runner", "hw", "Runner", "entry/runner", 0, "runner"),
    ("hw", "running", "hw", "Running", "entry/running", 0, "running"),
    ("hw", "jump", "hw", "Jump", "entry/jump", 0, "jump"),
    ("ex", "she runs every day", "ex", "She runs every day", "entry/ex1", 1, "runs"),
]


def _build_index(tmp_path):
    index_dir = str(tmp_path / "idx")
    maker = Maker(index_dir)
    for args in ITEMS:
        maker.add_item(*args)
    maker.commit()
    maker.close()
    return index_dir


def _build_var_cdb(tmp_path):
    """Build a tiny variations CDB: run -> {run, runs, ran}."""
    var_path = str(tmp_path / "var.cdb")
    with open(var_path, "w+b") as f:
        writer = VariationsWriter(f)
        writer.add("run", ["run", "runs", "ran"])
        writer.finalize()
    return var_path


# ---------------------------------------------------------------------------
# Maker / Searcher tests
# ---------------------------------------------------------------------------


class TestFulltextMakerSearcher:
    def test_basic_search(self, tmp_path):
        index_dir = _build_index(tmp_path)
        searcher = Searcher(index_dir, var_path="nonexistent_var.cdb")
        collector = searcher.make_collector(limit=20)
        results = searcher.search(collector, query_str1="run")
        searcher.close()
        labels = [r[0] for r in results]
        assert "Run" in labels

    def test_no_match(self, tmp_path):
        index_dir = _build_index(tmp_path)
        searcher = Searcher(index_dir, var_path="nonexistent_var.cdb")
        collector = searcher.make_collector(limit=20)
        results = searcher.search(collector, query_str1="zzz")
        searcher.close()
        assert results == []

    def test_itemtype_filter(self, tmp_path):
        index_dir = _build_index(tmp_path)
        searcher = Searcher(index_dir, var_path="nonexistent_var.cdb")
        collector = searcher.make_collector(limit=20)
        results = searcher.search(collector, query_str1="run", itemtypes=("ex",))
        searcher.close()
        # Only 'ex' typed items should appear
        for r in results:
            assert r[1].startswith("entry/ex")

    def test_result_structure(self, tmp_path):
        index_dir = _build_index(tmp_path)
        searcher = Searcher(index_dir, var_path="nonexistent_var.cdb")
        collector = searcher.make_collector(limit=20)
        results = searcher.search(collector, query_str1="jump")
        searcher.close()
        assert len(results) >= 1
        label, path, sortkey, prio, text = results[0]
        assert isinstance(label, str)
        assert isinstance(path, str)
        assert isinstance(sortkey, str)
        assert isinstance(prio, int)

    def test_wildcard_search(self, tmp_path):
        index_dir = _build_index(tmp_path)
        searcher = Searcher(index_dir, var_path="nonexistent_var.cdb")
        collector = searcher.make_collector(limit=20)
        results = searcher.search(collector, query_str1="run*")
        searcher.close()
        labels = [r[0] for r in results]
        # run, runner, running should all match
        assert len(labels) >= 2

    def test_reject_only_wildcard(self, tmp_path):
        index_dir = _build_index(tmp_path)
        searcher = Searcher(index_dir, var_path="nonexistent_var.cdb")
        collector = searcher.make_collector(limit=20)
        results = searcher.search(collector, query_str1="*")
        searcher.close()
        assert results == []

    def test_unlimited_collector(self, tmp_path):
        index_dir = _build_index(tmp_path)
        searcher = Searcher(index_dir, var_path="nonexistent_var.cdb")
        collector = searcher.make_collector()  # no limit
        results = searcher.search(collector, query_str1="run")
        searcher.close()
        assert len(results) >= 1


# ---------------------------------------------------------------------------
# VariationsReader / VariationsWriter tests
# ---------------------------------------------------------------------------


class TestVariationsRoundTrip:
    def test_get_variations(self, tmp_path):
        var_path = _build_var_cdb(tmp_path)
        reader = VariationsReader(var_path)
        variations = reader.get_variations("run")
        reader.close()
        assert "run" in variations
        assert "runs" in variations
        assert "ran" in variations

    def test_unknown_word_returns_singleton(self, tmp_path):
        var_path = _build_var_cdb(tmp_path)
        reader = VariationsReader(var_path)
        variations = reader.get_variations("xyz")
        reader.close()
        assert variations == {"xyz"}

    def test_searcher_with_variations(self, tmp_path):
        """Searcher that loads a real var CDB should expand query terms."""
        index_dir = _build_index(tmp_path)
        var_path = _build_var_cdb(tmp_path)
        searcher = Searcher(index_dir, var_path=var_path)
        collector = searcher.make_collector(limit=20)
        # searching for "ran" should match "run" via variations
        results = searcher.search(collector, query_str1="ran")
        searcher.close()
        labels = [r[0] for r in results]
        assert "Run" in labels
