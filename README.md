# Echo

## GameLift

* https://github.com/ZaMaZaN4iK/aws-gamelift-server-sdk-rs
* Test using GameLiftLocal
    * https://docs.aws.amazon.com/gamelift/latest/developerguide/integration-testing-local.html
    * java -jar GameLiftLocal.jar
        * Can override the port here if necessary, defaults to 8080
    * aws gamelift describe-game-sessions --endpoint-url http://localhost:8080 --fleet-id fleet-123
    * aws gamelift create-game-session --endpoint-url http://localhost:8080 --maximum-player-session-count 2 --fleet-id fleet-123
    * aws gamelift describe-instances --endpoint-url http://localhost:8080 --fleet-id fleet-123
* Requires musl target for building packages
  * Requires musl-tools to be installed
