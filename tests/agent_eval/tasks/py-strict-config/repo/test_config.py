import unittest

from config import hosts, port


class ConfigTest(unittest.TestCase):
    def test_reads_the_port(self):
        self.assertEqual(port({"port": 9000}), 9000)

    def test_reads_hosts(self):
        self.assertEqual(hosts({"hosts": ["a.example"]}), ["a.example"])


if __name__ == "__main__":
    unittest.main()
