"""Public niri B baseline shared by native composition and signed VM fixtures."""
import re


def incoming(original):
    published = original.replace('width 2', 'width 3')
    text = re.sub(r'(?m)^([ \t]*)gaps[ \t]+[0-9]+[ \t]*$', r'\g<1>gaps 18', published, count=1)
    return text.replace('\nbinds {\n', '\ncursor {\n    xcursor-size 28\n}\n\nbinds {\n')
