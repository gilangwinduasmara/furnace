import { useNavigate } from "react-router-dom";
import { Button } from "../components/ui/button";

export default function Welcome() {

    const navigate = useNavigate();
    return (
        <div className="flex flex-col items-center justify-center h-full space-y-24">
            <div className="text-center">
                <h1 className="text-2xl font-bold">Welcome to Furnace</h1>
                <p className="text-sm text-foreground/40">Powerful, hot, ready to cook your code.</p>
            </div>
            <div>
                <Button onClick={() => navigate("/recipes")} className="w-48">
                    Next
                </Button>
            </div>
        </div>
    );
}