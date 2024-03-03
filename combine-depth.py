depth = dict()

with open("depth.txt", "r") as f:
    for line in f:
        name, d = line.strip().split("=")
        depth[name] = int(d)
        print(name, d, sep="=")

with open("depth-2.txt", "r") as f:
    for line in f:
        name, d = line.strip().split("=")
        if int(d) < depth.get(name, 99):
            depth[name] = int(d)
            print(name, d, sep="=")

with open("depth-3.txt", "r") as f:
    for line in f:
        name, d = line.strip().split("=")
        if int(d) < depth.get(name, 99):
            depth[name] = int(d)
            print(name, d, sep="<=")
