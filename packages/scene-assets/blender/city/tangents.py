"""A GLB's tangents written again so the same mesh writes the same bytes."""
import json

import numpy as np


def steady_tangents(path):
    """Write every vertex's tangent again, from the first triangle that uses it: along
    that triangle's u, square to the vertex's normal. The exporter averages a smooth
    vertex's tangent over its faces in whatever order its threads finish, and rounds the
    sum, so one run in three wrote a rubble heap's tangent a ten-thousandth apart from the
    last. A flat face's tangent comes out as the exporter's own."""
    import struct

    data = bytearray(open(path, "rb").read())
    length = struct.unpack_from("<I", data, 12)[0]
    doc = json.loads(bytes(data[20:20 + length]))
    start = 20 + length + 8
    kind = {5121: "u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}

    def where(index):
        accessor = doc["accessors"][index]
        view = doc["bufferViews"][accessor["bufferView"]]
        if "byteStride" in view:
            raise SystemExit("steady_tangents: an interleaved buffer view")
        return start + view.get("byteOffset", 0) + accessor.get("byteOffset", 0), accessor["count"], width[accessor["type"]], \
            kind[accessor["componentType"]]

    def read(index):
        at, count, n, dtype = where(index)
        return np.frombuffer(bytes(data[at:at + count * n * np.dtype(dtype).itemsize]), dtype).reshape(count, n)

    for mesh in doc["meshes"]:
        for prim in mesh["primitives"]:
            attributes = prim["attributes"]
            if "TANGENT" not in attributes:
                continue
            p, n, uv = (read(attributes[k]).astype(np.float64) for k in ("POSITION", "NORMAL", "TEXCOORD_0"))
            t = read(prim["indices"]).astype(np.int64).reshape(-1, 3)
            e1, e2 = p[t[:, 1]] - p[t[:, 0]], p[t[:, 2]] - p[t[:, 0]]
            d1, d2 = uv[t[:, 1]] - uv[t[:, 0]], uv[t[:, 2]] - uv[t[:, 0]]
            det = d1[:, 0] * d2[:, 1] - d2[:, 0] * d1[:, 1]
            flat = np.abs(det) <= 1e-12
            safe = np.where(flat, 1.0, det)[:, None]
            along_u = (e1 * d2[:, 1:2] - e2 * d1[:, 1:2]) / safe
            along_v = (e2 * d1[:, 0:1] - e1 * d2[:, 0:1]) / safe
            first = np.full(len(p), len(t), dtype=np.int64)
            for corner in range(3):
                np.minimum.at(first, t[:, corner], np.arange(len(t)))
            first = np.minimum(first, len(t) - 1)
            tangent = along_u[first] - n * (n * along_u[first]).sum(1, keepdims=True)
            size = np.linalg.norm(tangent, axis=1, keepdims=True)
            lost = flat[first] | (size[:, 0] < 1e-9)
            # no u to follow (a triangle with no area in the texture): any direction square to the normal
            axis = np.eye(3)[np.abs(n).argmin(1)]
            spare = axis - n * (n * axis).sum(1, keepdims=True)
            tangent = np.where(lost[:, None], spare, tangent)
            tangent /= np.linalg.norm(tangent, axis=1, keepdims=True)
            # glTF's v runs down the image: the exporter's sign is the opposite of this one
            sign = np.where(lost, 1.0, -np.sign((np.cross(n, tangent) * along_v[first]).sum(1)))
            sign = np.where(sign == 0.0, 1.0, sign)
            out = np.concatenate([tangent, sign[:, None]], 1).astype("<f4")
            at, count, _, _ = where(attributes["TANGENT"])
            data[at:at + count * 16] = out.tobytes()
    open(path, "wb").write(bytes(data))
