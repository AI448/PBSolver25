from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import sys
import os
import utils
import subprocess
import time
import lzma


instance_list_file_path = Path(__file__).parent / "PB24_selected_DEC-LIN_instances.txt"
input_dir_path = Path(__file__).parent / ".." / "data" / "pbo"


def main():

    output_dir_path = Path(sys.argv[1])

    with open(instance_list_file_path) as instance_list_file:
        instance_list = [
            Path(instance + ".xz") for instance in instance_list_file.read().split("\n")
        ]

    os.makedirs(output_dir_path, exist_ok=True)

    for (input_file_path, status, seconds) in ThreadPoolExecutor(max_workers=8).map(execute, instance_list, [output_dir_path] * len(instance_list)):
        print(f"{input_file_path}\t{status}\t{seconds}", flush=True)
    return 0


def execute(instance_path: Path, output_dir_path: Path):
    input_file_path = input_dir_path / instance_path

    if not input_file_path.exists():
        raise Exception(f"Instance file \"{input_file_path}\" is not exist.")

    if not (output_dir_path / instance_path.parent).exists():
        os.makedirs(output_dir_path / instance_path.parent, exist_ok=True)

    log_file_path = output_dir_path / instance_path.parent / (str(instance_path.name.removesuffix(".opb.xz")) + ".txt")
    with lzma.open(input_file_path) as input_file:
        with open(log_file_path, "w") as log_file:
            data = input_file.read().decode()
            # print(input_file_path)
            # return (input_file_path, ["TEST", ""], 0)

            start_time = time.time()
            result = subprocess.run(
                ["../target/debug/solve_pb"],
                input=data,
                stdout=subprocess.PIPE,
                stderr=log_file,
                text=True,
            )
            seconds = time.time() - start_time
            if result.returncode == 0:
                status = [line for line in result.stdout.splitlines() if line.startswith("s ")]
                if len(status) == 0:
                    return (input_file_path, "FAILURE", seconds)
                else:
                    return (input_file_path, status[0][2:], seconds)
            else:
                return (input_file_path, "FAILURE", seconds)


if __name__ == "__main__":

    returncode = main()
    sys.exit(returncode)
