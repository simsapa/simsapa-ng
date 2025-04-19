#!/usr/bin/env python3

import json
import sys

def format_value(v):
    if isinstance(v, str):
        escaped = v.replace('\\', '\\\\').replace('"', '\\"')
        return f'"{escaped}"'
    elif isinstance(v, bool):
        return 'true' if v else 'false'
    elif isinstance(v, (int, float)):
        return str(v)
    else:
        escaped = str(v).replace('\\', '\\\\').replace('"', '\\"')
        return f'"{escaped}"'

def main():
    if len(sys.argv) != 3:
        print("Usage: python json_to_qml.py input.json output.qml")
        sys.exit(1)

    input_file, output_file = sys.argv[1], sys.argv[2]

    with open(input_file, 'r') as f:
        data = json.load(f)

    qml_content = ['import QtQuick\n', 'ListModel {']

    for item in data:
        qml_content.append('    ListElement {')
        for key, value in item.items():
            if value is None:  # Skip null values
                continue
            qml_content.append(f'        {key}: {format_value(value)}')
        qml_content.append('    }')

    qml_content.append('}')

    with open(output_file, 'w') as f:
        f.write('\n'.join(qml_content))

if __name__ == '__main__':
    main()
