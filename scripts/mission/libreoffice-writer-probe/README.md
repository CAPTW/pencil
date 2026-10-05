# Isolated Writer driver

The actual transaction core is adapters/libreoffice/pythonpath/grammar_writer_core.py.
This directory contains only the guarded UNO driver, deliberately adversarial test
listeners, and fixture manifest. Neither this service nor its hooks may ship.

Build with scripts/mission/build-libreoffice-writer-probe.py --output <fresh.oxt>.
It includes the actual core in pythonpath/ and prints the exact source hashes.
Use only the existing offline restricted-Medium Windows Sandbox launcher and
synthetic client; normal user documents/profiles/Providers remain out of scope.

See adapters/libreoffice/README.md and docs/control/HANDOFF.md for precise
qualification, retained failures, privacy limits and product activation work.
The earlier failing setString/Undo callback candidate remains in
D:/dev/grammar-evidence/grammar-libreoffice-writer-20261005.
The custom Undo implementation and subsequent evidence are in
D:/dev/grammar-evidence/grammar-writer-activation-20261005.
