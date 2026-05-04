"""Tests for ldoce5viewer.utils.cdb (CDBMaker / CDBReader)."""

import pytest

from ldoce5viewer.utils.cdb import CDBError, CDBMaker, CDBReader


class TestCDBRoundTrip:
    def test_single_entry(self, tmp_path):
        path = str(tmp_path / "test.cdb")
        with open(path, "w+b") as f:
            maker = CDBMaker(f)
            maker.add(b"hello", b"world")
            maker.finalize()

        with CDBReader(path) as reader:
            assert reader[b"hello"] == b"world"

    def test_multiple_entries(self, tmp_path):
        items = [(b"key%d" % i, b"val%d" % i) for i in range(50)]
        path = str(tmp_path / "multi.cdb")
        with open(path, "w+b") as f:
            maker = CDBMaker(f)
            for k, v in items:
                maker.add(k, v)
            maker.finalize()

        with CDBReader(path) as reader:
            for k, v in items:
                assert reader[k] == v

    def test_missing_key_returns_default(self, tmp_path):
        path = str(tmp_path / "miss.cdb")
        with open(path, "w+b") as f:
            maker = CDBMaker(f)
            maker.add(b"exists", b"yes")
            maker.finalize()

        with CDBReader(path) as reader:
            assert reader.get(b"nope") is None
            assert reader.get(b"nope", b"default") == b"default"

    def test_key_error_on_getitem(self, tmp_path):
        path = str(tmp_path / "kerr.cdb")
        with open(path, "w+b") as f:
            maker = CDBMaker(f)
            maker.add(b"a", b"b")
            maker.finalize()

        with CDBReader(path) as reader:
            with pytest.raises(KeyError):
                _ = reader[b"z"]

    def test_contains(self, tmp_path):
        path = str(tmp_path / "cont.cdb")
        with open(path, "w+b") as f:
            maker = CDBMaker(f)
            maker.add(b"present", b"1")
            maker.finalize()

        with CDBReader(path) as reader:
            assert b"present" in reader
            assert b"absent" not in reader

    def test_iteritems(self, tmp_path):
        items = {b"k1": b"v1", b"k2": b"v2", b"k3": b"v3"}
        path = str(tmp_path / "iter.cdb")
        with open(path, "w+b") as f:
            maker = CDBMaker(f)
            for k, v in items.items():
                maker.add(k, v)
            maker.finalize()

        with CDBReader(path) as reader:
            result = dict(reader.iteritems())
        assert result == items

    def test_empty_value(self, tmp_path):
        path = str(tmp_path / "empty_val.cdb")
        with open(path, "w+b") as f:
            maker = CDBMaker(f)
            maker.add(b"k", b"")
            maker.finalize()

        with CDBReader(path) as reader:
            assert reader[b"k"] == b""

    def test_binary_key_and_value(self, tmp_path):
        key = bytes(range(256))
        value = bytes(reversed(range(256)))
        path = str(tmp_path / "binary.cdb")
        with open(path, "w+b") as f:
            maker = CDBMaker(f)
            maker.add(key, value)
            maker.finalize()

        with CDBReader(path) as reader:
            assert reader[key] == value

    def test_file_too_small_raises(self, tmp_path):
        path = str(tmp_path / "small.cdb")
        path_obj = tmp_path / "small.cdb"
        path_obj.write_bytes(b"\x00" * 10)
        with pytest.raises(CDBError):
            CDBReader(path)
