"""Extract baseline markers with Python 3's standard-library HTML parser.

Only real head canonical links and body demo sections count. Text, comments,
script/style contents and inert templates are not homepage identity markers.
"""
import json
import sys
from html.parser import HTMLParser


class BaselineParser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.in_head = False
        self.in_body = False
        self.inert = 0
        self.raw = None
        self.canonicals = []
        self.demo = False

    def handle_starttag(self, tag, attrs):
        if self.raw:
            return
        if tag in {"title", "textarea", "xmp", "iframe", "noembed", "noframes", "noscript", "plaintext"}:
            self.raw = tag
            return
        if tag == "template":
            self.inert += 1
        if self.inert:
            return
        # Duplicate attributes are ambiguous; never accept them as identity.
        if len({key for key, _ in attrs}) != len(attrs):
            return
        attrs = dict(attrs)
        if tag == "head":
            self.in_head = True
        elif tag == "body":
            self.in_body = True
        elif tag == "link" and self.in_head and "canonical" in (attrs.get("rel") or "").lower().split():
            self.canonicals.append(attrs.get("href"))
        elif tag == "section" and self.in_body and "pond-demo" in (attrs.get("class") or "").split():
            self.demo = True

    def handle_endtag(self, tag):
        if self.raw:
            if tag == self.raw:
                self.raw = None
            return
        if tag == "template" and self.inert:
            self.inert -= 1
        elif not self.inert:
            if tag == "head":
                self.in_head = False
            elif tag == "body":
                self.in_body = False


parser = BaselineParser()
parser.feed(sys.stdin.read())
parser.close()
print(json.dumps({"canonicals": parser.canonicals, "demo": parser.demo}))
