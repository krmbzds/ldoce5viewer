"""Minimal Python 3 compatibility shim (Python 2 support has been dropped).

These names are re-exported so that legacy ``from .compat import …`` statements
in other modules continue to work without modification.
"""

basestring = str
range = range
zip = zip
