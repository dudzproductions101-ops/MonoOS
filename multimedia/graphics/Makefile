CXX      := g++
CXXFLAGS := -std=c++17 -O2 -Wall -fPIC
LDFLAGS  := -shared

TARGET := libmonoos_gfx.so
SRCS   := vulkan_context.cpp
OBJS   := $(SRCS:.cpp=.o)

all: $(TARGET)
$(TARGET): $(OBJS)
	$(CXX) $(LDFLAGS) -o $@ $^
%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@
clean:
	rm -f $(OBJS) $(TARGET)
