# Deja Dup Auto Ignore

Automatically creates `.deja-dup-ignore` or `CACHEDIR.TAG` files for directories that shouldn't be backed up by Deja Dup, but hasn't been (i.e. not in Deja Dup's exclude list and doesn't contain the given mark files).

The criteria for creating the files:
- it's a directory ignored by `.gitignore`, and
- the directory has specific name.

Specific names for different files:
<table>
<tr>
<th colspan="2"><code>.deja-dup-ignore</code></th>
</tr>
<tr>
<td><code>node_modules</code></td>
<td>Dependencies for JavaScript projects</td>
</tr>
<tr>
<td><code>venv</code>, <code>.venv</code></td>
<td>Python virtual environment</td>
</tr>
<tr>
<td><code>.gradle</code></td>
<td>Build output for Gradle projects</td>
</tr>
<tr>
<td><code>target</code></td>
<td>Build output for Rust projects</td>
</tr>
<tr>
<td><code>build</code>, <code>out</code>, <code>dist</code></td>
<td>Build output for other projects</td>
</tr>
<tr>
<th colspan="2"><code>CACHEDIR.TAG</code></th>
</tr>
<tr>
<td>Any name containing <code>cache</code></td>
<td>Cache directory</td>
</tr>
</table>

## License

    Copyright (C) 2025  Xidorn Quan

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.