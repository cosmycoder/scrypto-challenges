import DefinedButton from "./components/Button";
import { AiFillCaretRight } from "react-icons/ai";

export default function Home() {
  return (
    <div>
      <head>
        <title>Defi Saving</title>
        <meta name="description" content="Generated by create next app" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <link rel="icon" href="/favicon.ico" />
      </head>
      <header>
        <div className="flex items-center w-full justify-between p-4 bg-gray-900  ">
          <p className="font-bold text-xl">🦄 DeCrypt </p>
          <DefinedButton text="Connect Wallet" variant="primary" />
        </div>
      </header>
      <main>
        <div className="text-center my-20 mx-4">
          <h1 className="text-6xl font-bold my-2 text-white capitalize">
            Invest in a new <span className="text-primary-1"> Way</span> ,in a{" "}
            <span className="text-primary-1"> new World</span>
          </h1>

          <p className="py-3">
            {" "}
            Invest beyond borders, <br /> We connect across cryptocurrency
            platforms around the world.{" "}
          </p>
          <div className="flex items-center justify-center gap-2">
            <a href="/dashboard">
              <DefinedButton text="Get Started &rarr;" variant="primary" />
            </a>
            <DefinedButton
              text={
                <>
                  {" "}
                  See how it works <AiFillCaretRight />{" "}
                </>
              }
            />
          </div>
        </div>
        <div className=""></div>
      </main>
    </div>
  );
}
